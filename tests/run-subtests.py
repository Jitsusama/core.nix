"""Run some of a VM test's subtests, in seconds rather than minutes.

A VM test's driver builds in seconds once its machines are built, and runs
whatever script it's given. This builds the check's driver, cuts its script
down to everything outside a subtest (the boot, the helpers) and the
subtests asked for, and runs that in a runtime folder of its own, so two
runs at once don't share the driver's network switch or machine state.

    run-subtests [options] <check> [<words>...]

Each subtest whose name holds any of the words runs, in the script's order;
with none, the subtests are listed. A subtest that uses what an earlier one
left, a window or a notice, needs that one named too. The pictures are kept
in the folder it prints at the end.
"""

import argparse
import ast
import contextlib
import os
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Iterator, Sequence

# The driver puts its network switch's socket a few names under
# XDG_RUNTIME_DIR, and a socket's path can't pass 108 bytes, so a runtime
# folder longer than this is swapped for one under /tmp.
LONGEST_RUNTIME_BASE = 40

# What a run asked for is refused with, as sysexits.h numbers it.
USAGE = 64

type Chunk = tuple[str | None, list[str]]


@dataclass(frozen=True)
class Options:
    """What the command line asked for."""

    check: str
    words: list[str]
    flake: str
    out: Path | None
    interactive: bool
    keep_machine_state: bool


def subtest_name(statement: ast.stmt) -> str | None:
    """Give the name a `with subtest("..."):` statement gives, or None."""
    if not isinstance(statement, ast.With) or len(statement.items) != 1:
        return None
    call = statement.items[0].context_expr
    if (
        isinstance(call, ast.Call)
        and isinstance(call.func, ast.Name)
        and call.func.id == "subtest"
        and len(call.args) == 1
        and isinstance(call.args[0], ast.Constant)
        and isinstance(call.args[0].value, str)
    ):
        return call.args[0].value
    return None


def chunks(script: str) -> list[Chunk]:
    """Split a test script into its top-level statements.

    Each is a name and its lines: the subtest's name for a subtest, None for
    anything else. Python parses it, so a string at the margin stays in its
    statement, and each starts where the one before it ended, so the
    comments and decorators before a statement go with it.
    """
    lines = script.splitlines(keepends=True)
    body = ast.parse(script).body
    starts = [0] + [statement.end_lineno or statement.lineno for statement in body[:-1]]
    ends = [*starts[1:], len(lines)]
    return [
        (subtest_name(statement), lines[start:end])
        for statement, start, end in zip(body, starts, ends, strict=True)
    ]


def named(name: str, words: Sequence[str]) -> bool:
    """Say whether a subtest's name holds any of the words."""
    return any(word in name for word in words)


def names(script: str) -> list[str]:
    """List the script's subtests, in the order they run."""
    return [name for name, _ in chunks(script) if name is not None]


def cut(script: str, words: Sequence[str]) -> str:
    """Cut the script down to the subtests the words name.

    Everything outside a subtest is kept.
    """
    return "".join(
        line
        for name, lines in chunks(script)
        if name is None or named(name, words)
        for line in lines
    )


def nix(*arguments: str) -> str:
    """Run nix, the one on the path, and give what it printed."""
    # The nix run is the person's own, with their settings, so it's found
    # on the path rather than pinned; what it's given is theirs too.
    return subprocess.run(  # noqa: S603
        ["nix", *arguments],  # noqa: S607
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    ).stdout.strip()


def build_driver(flake: str, check: str) -> Path:
    """Build the check's driver, for this machine's system."""
    system = nix("eval", "--raw", "--impure", "--expr", "builtins.currentSystem")
    target = f"{flake}#checks.{system}.{check}.driver"
    print(f"building {target}", file=sys.stderr)
    return Path(nix("build", "--no-link", "--print-out-paths", target))


@contextlib.contextmanager
def runtime_folder(check: str, *, keep: bool) -> Iterator[Path]:
    """Give a short runtime folder for one run.

    Kept machines live in one named for the check under the person's own
    runtime folder, so the check's next run finds them. Otherwise each run
    has a new one, under /tmp when the person's is too long, gone when the
    run ends.
    """
    base = os.environ.get("XDG_RUNTIME_DIR", "")
    short = 0 < len(base) <= LONGEST_RUNTIME_BASE
    if keep:
        if not short:
            sys.exit(f"keeping machines needs a short XDG_RUNTIME_DIR, not {base!r}")
        folder = Path(base) / f"vm.{check}"
        folder.mkdir(mode=0o700, exist_ok=True)
        yield folder
        return
    # mkdtemp makes a new folder only its owner can enter, so /tmp is safe.
    folder = Path(tempfile.mkdtemp(prefix="vm.", dir=base if short else "/tmp"))  # noqa: S108
    try:
        yield folder
    finally:
        shutil.rmtree(folder, ignore_errors=True)


def run(driver: Path, script: str, out: Path, options: Options) -> int:
    """Run the script on the driver, its pictures kept in out."""
    with runtime_folder(options.check, keep=options.keep_machine_state) as runtime:
        script_path = runtime / "test-script.py"
        script_path.write_text(script)
        command = [
            str(driver / "bin" / "nixos-test-driver"),
            "--test-script",
            str(script_path),
            "--output_directory",
            str(out),
        ]
        if options.interactive:
            command.append("--interactive")
        if options.keep_machine_state:
            command.append("--keep-machine-state")
        # A failing test is reported by its status, not raised.
        return subprocess.run(  # noqa: S603
            command,
            env={**os.environ, "XDG_RUNTIME_DIR": str(runtime)},
            check=False,
        ).returncode


def arguments(argv: Sequence[str]) -> Options:
    """Read what the command line asks for."""
    parser = argparse.ArgumentParser(
        prog="run-subtests",
        description="Run some of a VM test's subtests on its driver.",
    )
    parser.add_argument("check", help="the VM test, e.g. vm-desktop-works")
    parser.add_argument("words", nargs="*", help="words a subtest's name holds")
    parser.add_argument("--flake", default=".", help="the flake, . by default")
    parser.add_argument("--out", type=Path, help="where the pictures go, a new folder by default")
    parser.add_argument(
        "-I",
        "--interactive",
        action="store_true",
        help="a Python prompt once the script has run, machines up",
    )
    parser.add_argument(
        "-K",
        "--keep-machine-state",
        action="store_true",
        help="keep the machines' disks for the next run of the check",
    )
    parsed = parser.parse_args(argv)
    return Options(
        check=parsed.check,
        words=parsed.words,
        flake=parsed.flake,
        out=parsed.out,
        interactive=parsed.interactive,
        keep_machine_state=parsed.keep_machine_state,
    )


def main(argv: Sequence[str]) -> int:
    """List a check's subtests, or run those the words name."""
    options = arguments(argv)
    driver = build_driver(options.flake, options.check)
    script = (driver / "test-script").read_text()
    if not options.words:
        print("\n".join(names(script)))
        return 0
    chosen = [name for name in names(script) if named(name, options.words)]
    if not chosen:
        print(f"no subtest of {options.check} is named by {options.words}", file=sys.stderr)
        return USAGE
    print("running:\n  " + "\n  ".join(chosen), file=sys.stderr)
    out = options.out or Path(tempfile.mkdtemp(prefix=f"{options.check}."))
    out.mkdir(parents=True, exist_ok=True)
    status = run(driver, cut(script, options.words), out, options)
    print(f"pictures: {out}", file=sys.stderr)
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
