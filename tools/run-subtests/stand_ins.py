"""Stand-ins run-subtests' tests run it against, and what they use of it.

The tool runs nix, nix-offload, git and a check's driver; each is a small
program here that notes what it was handed and answers as a test tells it,
through the environment, so a test can say what the tool was asked and how
it took the answer. The tool itself is the file RUN_SUBTESTS names, since
it's a program rather than a module; conftest.py loads it.
"""

import json
import os
import sys
from typing import TYPE_CHECKING, Protocol, cast

if TYPE_CHECKING:
    from collections.abc import Sequence
    from pathlib import Path

    import pytest

# A script the driver runs top to bottom, as VM tests were written before
# pytest, which has no tests to pick.
SCRIPT = 'start_all()\nwith subtest("the bar shows"):\n    machine.succeed("true")\n'

# A check's script as a pytest-driven VM test's testScript writes it.
PYTEST_SCRIPT = 'import vm_pytest\nvm_pytest.run(globals(), "/nix/store/x-signing-vm-test.py")\n'

# A stand-in for nix: this machine's system, and the driver it "built".
NIX = """
import os, sys
if sys.argv[1] == "eval":
    print("x86_64-linux")
else:
    print(os.environ["STAND_IN_DRIVER"])
"""

# A stand-in for a check's driver: it notes the script it was handed, the
# options pytest would read, which go with the run's folder, and how it was
# run. Asked for a listing, it lists two tests as pytest's quiet listing
# does, past a warning and a byte no text reads, or as -v lists them when
# told; and it fails as told.
DRIVER = """
import json, os, sys
script = open(sys.argv[sys.argv.index("--test-script") + 1]).read()
with open(os.environ["STAND_IN_RAN"], "w") as ran:
    json.dump({
        "script": script,
        "pytest": os.environ.get("PYTEST_ADDOPTS"),
        "unbuffered": os.environ.get("PYTHONUNBUFFERED"),
        "interactive": "--interactive" in sys.argv,
    }, ran)
if "--collect-only" in os.environ.get("PYTEST_ADDOPTS", ""):
    if os.environ.get("STAND_IN_LISTS_VERBOSE"):
        print("<Module signing-vm-test.py>")
        print("  <Function test_terminal>")
    else:
        print("vm_pytest.py:12: UserWarning: a warning before the names", flush=True)
        sys.stdout.buffer.write(b"a byte no text reads: \\xff\\n")
        sys.stdout.buffer.flush()
        print("signing-vm-test.py::test_terminal")
        print("signing-vm-test.py::test_refused[the password]")
    print()
    print("2 tests collected in 0.01s")
sys.exit(int(os.environ.get("STAND_IN_STATUS", "0")))
"""


# A stand-in for nix-offload: it notes what it was asked, puts a picture in
# the folder it was given unless told the run took none, and answers as told.
NIX_OFFLOAD = """
import json, os, sys
with open(os.environ["STAND_IN_ASKED"], "w") as asked:
    json.dump(sys.argv[1:], asked)
out = sys.argv[sys.argv.index("--out") + 1]
if os.environ.get("STAND_IN_OFFLOAD_SAYS"):
    print(os.environ["STAND_IN_OFFLOAD_SAYS"], file=sys.stderr)
if os.environ.get("STAND_IN_OFFLOAD_UNREADABLE"):
    sys.stderr.buffer.write(b"a byte no text reads: \\xff\\n")
    sys.stderr.buffer.flush()
if not os.environ.get("STAND_IN_NO_PICTURES"):
    open(os.path.join(out, "desktop.png"), "w").close()
if os.environ.get("STAND_IN_OFFLOAD_LISTS"):
    with open(os.path.join(out, "subtests"), "w") as listed:
        listed.write(os.environ["STAND_IN_OFFLOAD_LISTS"])
sys.exit(int(os.environ.get("STAND_IN_OFFLOAD_STATUS", "0")))
"""


# A stand-in for git: origin main's quarantine list is the file
# STAND_IN_QUARANTINE names, and without one main has no list.
GIT = """
import os, sys
listed = os.environ.get("STAND_IN_QUARANTINE")
if sys.argv[3:] != ["show", "refs/remotes/origin/main:tools/merge-net/quarantine.json"]:
    sys.exit(2)
if not listed:
    sys.exit(128)
print(open(listed).read())
"""


class Tool(Protocol):
    """What the tests use of run-subtests."""

    def pytest_driven(self, script: str) -> bool:
        """Say whether the script hands its tests to pytest."""

    def selecting(self, words: Sequence[str]) -> str:
        """Give the pytest options that pick the tests the words name."""

    def main(self, argv: Sequence[str]) -> int:
        """List a check's subtests, or run those the words name."""

    def nix_string(self, text: str) -> str:
        """Write text as a Nix string."""

    @property
    def USAGE(self) -> int:  # noqa: N802 the module's constant, read through the protocol
        """What a run asked for wrongly is refused with."""


def program(path: Path, source: str) -> None:
    """Write a Python program at path, run by this Python wherever it's started."""
    path.write_text(f"#!{sys.executable}\n{source}")
    path.chmod(0o755)


class StandIn:
    """A check's driver and the nix that builds it, first on the path."""

    def __init__(self, folder: Path, runtime: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        """Lay the stand-ins out in folder, and put them first on the path.

        The runtime folder is short, as the tool asks of one it runs a driver under.
        """
        self.folder = folder
        driver = folder / "driver"
        (driver / "bin").mkdir(parents=True)
        (folder / "bin").mkdir()
        program(folder / "bin" / "nix", NIX)
        program(folder / "bin" / "nix-offload", NIX_OFFLOAD)
        program(folder / "bin" / "git", GIT)
        program(driver / "bin" / "nixos-test-driver", DRIVER)
        self.driver = driver
        self.script = driver / "test-script"
        monkeypatch.setenv("STAND_IN_ASKED", str(folder / "asked.json"))
        monkeypatch.setenv("PATH", f"{folder / 'bin'}{os.pathsep}{os.environ['PATH']}")
        monkeypatch.setenv("STAND_IN_DRIVER", str(driver))
        monkeypatch.setenv("STAND_IN_RAN", str(folder / "ran.json"))
        monkeypatch.setenv("XDG_RUNTIME_DIR", str(runtime))
        monkeypatch.delenv("PYTEST_ADDOPTS", raising=False)
        monkeypatch.delenv("STAND_IN_QUARANTINE", raising=False)
        monkeypatch.delenv("DIRENV_DIR", raising=False)

    def ran(self) -> dict[str, object] | None:
        """Give how the driver was run, or None if it wasn't."""
        ran = self.folder / "ran.json"
        return cast("dict[str, object]", json.loads(ran.read_text())) if ran.exists() else None

    def asked(self) -> list[str] | None:
        """Give what nix-offload was asked, or None if it wasn't."""
        asked = self.folder / "asked.json"
        return cast("list[str]", json.loads(asked.read_text())) if asked.exists() else None

    def ran_script(self) -> str:
        """Give the script the driver was handed."""
        ran = self.ran()
        assert ran is not None, "the driver never ran"
        return cast("str", ran["script"])
