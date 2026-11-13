"""Run some of a VM test's subtests, in seconds rather than minutes.

A VM test's driver builds in seconds once its machines are built, and runs
whatever script it's given. This builds the check's driver, cuts its script
down to everything outside a subtest (the boot, the helpers) and the
subtests asked for, and runs that in a runtime folder of its own, so two
runs at once don't share the driver's network switch or machine state.

    run-subtests [options] <check> [<words>...]

Each subtest whose name holds any of the words runs, in the script's order;
with none, the subtests are listed. Each subtest makes what it needs when an
earlier one hasn't, so any runs alone. The pictures are kept
in the folder it prints at the end.

A test written for pytest (tests/lib/vm_pytest.py) is run whole instead, and
pytest picks the tests: each word, its spaces read as underscores, is matched
against the tests' names, and with none they're listed, which boots nothing.

With --there, all of it is built elsewhere: nix-offload builds the flake's
`subtests` for the check and words where the VM can run (subtests.nix),
which runs this same program on the check's driver there, and copies the
pictures back. An nix-offload that can't take that runs it here.

A check the net holds in quarantine on origin's main
(tools/merge-net/quarantine.json) still runs, but its failure is said and
blocks nothing: the run exits 0. An entry naming subtests excuses only a run
built elsewhere whose every failed subtest it names, as the net judges one.
"""

import argparse
import ast
import contextlib
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, cast

if TYPE_CHECKING:
    from collections.abc import Iterable, Iterator, Sequence

# The driver puts its network switch's socket a few names under
# XDG_RUNTIME_DIR, and a socket's path can't pass 108 bytes, so a runtime
# folder longer than this is swapped for one under /tmp.
LONGEST_RUNTIME_BASE = 40

# What a run asked for is refused with, as sysexits.h numbers it.
USAGE = 64

# What a run that ran and failed exits with: pytest's and nix's own failure,
# and Nix's for a build that failed. Anything else, such as nix-offload's 75
# when no builder took it, means nothing ran, so nothing failed that a
# quarantine could excuse: between 01:30 and 04:15 on 8 November runs that
# never ran read as passing.
FAILED = frozenset({1, 100})

# What a word may hold to be matched against a pytest test's name, as
# pytest's -k reads a name, so no word is read as more of the expression.
KEYWORD = re.compile(r"[A-Za-z0-9_ -]+")
# The words -k reads as its own, which a word may not be alone.
OPERATORS = frozenset({"and", "or", "not"})

# What subtests.nix says of a check that isn't a VM test, and so has no
# subtests: its evaluation fails, which must never read as the check failing.
NOT_A_VM_TEST = "isn't a VM test, so it has no subtests"

# Where the net keeps its quarantine list, and the main it's read from, so a
# branch can't quarantine its own failure.
QUARANTINE = "tools/merge-net/quarantine.json"
MAIN = "refs/remotes/origin/main"

# A failed pytest test as a build's log says it, read as merge-net's
# subtests.rs reads it: by its function's name, its parameters dropped.
FAILED_TEST = re.compile(r"FAILED \S*::(test_\w+)")

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
    driver: Path | None = None
    there: bool = False


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


def pytest_driven(script: str) -> bool:
    """Say whether the script hands its tests to pytest, through vm_pytest.run."""
    return any(
        isinstance(statement, ast.Expr)
        and isinstance(statement.value, ast.Call)
        and isinstance(statement.value.func, ast.Attribute)
        and isinstance(statement.value.func.value, ast.Name)
        and statement.value.func.value.id == "vm_pytest"
        and statement.value.func.attr == "run"
        for statement in ast.parse(script).body
    )


def selecting(words: Sequence[str]) -> str:
    """Give the pytest options that pick the tests whose names hold any of the words.

    A test's name is its function's, so a word's spaces are read as underscores.
    """
    keywords = ["_".join(word.split()) for word in words]
    for word, keyword in zip(words, keywords, strict=True):
        if not KEYWORD.fullmatch(word) or not keyword or keyword.lower() in OPERATORS:
            message = f"{word!r} can't be matched against a test's name"
            raise ValueError(message)
    return "-k " + shlex.quote(" or ".join(keywords))


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
    # A Nix build has no runtime folder and maybe no /tmp, but a short TMPDIR.
    build = os.environ.get("TMPDIR", "")
    fallback = build if 0 < len(build) <= LONGEST_RUNTIME_BASE else "/tmp"  # noqa: S108
    folder = Path(tempfile.mkdtemp(prefix="vm.", dir=base if short else fallback))
    try:
        yield folder
    finally:
        shutil.rmtree(folder, ignore_errors=True)


def run(driver: Path, script: str, out: Path, options: Options, pytest: str = "") -> int:
    """Run the script on the driver, its pictures kept in out, pytest given those options."""
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
        environment = {**os.environ, "XDG_RUNTIME_DIR": str(runtime)}
        if pytest:
            # After any the person set, so theirs hold too.
            given = os.environ.get("PYTEST_ADDOPTS", "")
            environment["PYTEST_ADDOPTS"] = f"{given} {pytest}".strip()
        return subprocess.run(  # noqa: S603
            command,
            env=environment,
            check=False,
        ).returncode


def arguments(argv: Sequence[str]) -> Options:
    """Read what the command line asks for."""
    parser = argparse.ArgumentParser(
        prog="run-subtests",
        description="Run some of a VM test's subtests on its driver.",
    )
    parser.add_argument("check", help="the VM test, e.g. vm-desktop-starts-clean")
    parser.add_argument("words", nargs="*", help="words a subtest's name holds")
    parser.add_argument("--flake", help="the flake, . by default")
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
    parser.add_argument(
        "--there",
        action="store_true",
        help="build and run it where nix-offload builds, the pictures copied back",
    )
    parser.add_argument("--driver", type=Path, help="the check's driver, already built")
    parsed = parser.parse_args(argv)
    if parsed.there and (parsed.interactive or parsed.keep_machine_state):
        parser.error("--there runs no prompt and keeps no machines")
    loaded = another_tree()
    if parsed.flake is None and loaded is not None:
        parser.error(
            f"the shell loaded is {loaded}'s, but this runs in {Path.cwd()}, whose code it "
            f"would test; cd to {loaded}, or name the one meant with --flake"
        )
    return Options(
        check=parsed.check,
        words=parsed.words,
        flake=parsed.flake or ".",
        out=parsed.out,
        interactive=parsed.interactive,
        keep_machine_state=parsed.keep_machine_state,
        driver=parsed.driver,
        there=parsed.there,
    )


def another_tree() -> Path | None:
    """Give the folder whose shell direnv loaded, when this folder isn't in it.

    `direnv exec TREE run-subtests` loads TREE's shell and stays where it is,
    so from the main checkout it tests main: on 8 November runs meant for
    branches did, and their passes were kept at main's commit.
    """
    loaded = os.environ.get("DIRENV_DIR", "").removeprefix("-")
    if not loaded:
        return None
    folder = Path(loaded).resolve()
    return None if Path.cwd().resolve().is_relative_to(folder) else folder


def nix_string(text: str) -> str:
    """Write text as a Nix string, which reads JSON's escapes but not `${`."""
    return json.dumps(text).replace("${", "\\${")


def failed_tests(lines: Iterable[str]) -> frozenset[str]:
    """Give the pytest tests a build's log says failed, by their functions' names."""
    return frozenset(
        found.group(1) for line in lines if (found := FAILED_TEST.search(line)) is not None
    )


def there(options: Options) -> tuple[int, frozenset[str]] | None:
    """Have nix-offload build and run it elsewhere, or None when it can't.

    It gives the build's status and the tests its log says failed.

    nix-offload refuses arguments it doesn't know with USAGE, as one too old
    for --apply and --out does, and one that isn't installed can't be run.
    """
    system = nix("eval", "--raw", "--impure", "--expr", "builtins.currentSystem")
    words = " ".join(nix_string(word) for word in options.words)
    apply = f"subtests: subtests {{ check = {nix_string(options.check)}; words = [ {words} ]; }}"
    flake = options.flake
    if ":" not in flake:
        flake = str(Path(flake).resolve())
    out = options.out or Path(tempfile.mkdtemp(prefix=f"{options.check}."))
    out.mkdir(parents=True, exist_ok=True)
    command = ["nix-offload", "there", "-L", "--out", str(out), "--apply", apply]
    print(f"building {options.check}'s subtests where nix-offload builds", file=sys.stderr)
    try:
        status, refused, failed = offloaded([*command, flake, f"legacyPackages.{system}.subtests"])
    except FileNotFoundError:
        return None
    if refused:
        print(
            f"run-subtests: {options.check} isn't a VM test; build it whole with "
            f"nix-offload there . checks.{system}.{options.check}",
            file=sys.stderr,
        )
        return USAGE, frozenset()
    if status == USAGE:
        return None
    listed = out / "subtests"
    if not options.words and listed.exists():
        print(listed.read_text(), end="")
    else:
        say_pictures(out, options.check)
    return status, failed


def offloaded(command: list[str]) -> tuple[int, bool, frozenset[str]]:
    """Run nix-offload, and give its status, whether it refused the check, and what failed.

    Its errors are shown as they come, and read for subtests.nix's refusal
    of a check that isn't a VM test and for the tests the build says failed.
    """
    refused = False
    said: list[str] = []
    with subprocess.Popen(command, stderr=subprocess.PIPE, text=True) as running:  # noqa: S603
        for line in running.stderr or ():
            sys.stderr.write(line)
            refused = refused or NOT_A_VM_TEST in line
            said.append(line)
    return running.returncode, refused, failed_tests(said)


def say_pictures(out: Path, check: str) -> None:
    """Say where the run's pictures are, or that it took none.

    A check shoots only as its gallery: on 5 November a passing run of
    vm-desktop-looks brought back an empty folder, read as pictures lost.
    """
    if any(entry.name != "subtests" for entry in out.iterdir()):
        print(f"pictures: {out}", file=sys.stderr)
        return
    gallery = "" if check.endswith(".gallery") else f"; {check}.gallery takes them, if it has one"
    print(f"no pictures: the subtests run took none{gallery}", file=sys.stderr)


def quarantined(flake: str, check: str) -> dict[str, object] | None:
    """Give origin main's quarantine entry for the check, or None.

    A gallery, `vm-a.gallery`, is held with its test. A repository without
    the list, or without git, quarantines nothing, and a list that doesn't
    read is said so and quarantines nothing either.
    """
    folder = "." if ":" in flake else flake
    try:
        # git is the person's own, found on the path, as nix is.
        shown = subprocess.run(  # noqa: S603
            ["git", "-C", folder, "show", f"{MAIN}:{QUARANTINE}"],  # noqa: S607
            check=False,
            capture_output=True,
            text=True,
        )
    except FileNotFoundError:
        return None
    if shown.returncode != 0:
        return None
    try:
        entries: object = json.loads(shown.stdout)
    except json.JSONDecodeError as error:
        print(f"run-subtests: {QUARANTINE} on main doesn't read: {error}", file=sys.stderr)
        return None
    test = check.split(".", maxsplit=1)[0]
    for entry in entries if isinstance(entries, list) else []:
        if isinstance(entry, dict) and entry.get("test") == test:
            return cast("dict[str, object]", entry)
    return None


def excused(held: dict[str, object], failed: frozenset[str]) -> bool:
    """Say whether a quarantine entry excuses a run in which those tests failed.

    A whole entry excuses any failure. One naming subtests excuses a run
    only when it names every test that failed, and never a run that names
    none, such as one that timed out or ran here, as merge-net judges it.
    """
    listed = listed_subtests(held)
    return listed is None or (bool(failed) and failed <= listed)


def listed_subtests(held: dict[str, object]) -> frozenset[str] | None:
    """Give the subtests a quarantine entry names, or None for a whole entry."""
    listed = held.get("subtests")
    if not isinstance(listed, list):
        return None
    return frozenset(name for name in cast("list[object]", listed) if isinstance(name, str))


def blocking(options: Options, status: int, failed: frozenset[str]) -> int:
    """Give the run's status, a failure main's quarantine excuses read as none.

    Inside a build, given its driver, the status is the build's own, and
    the run that sent it there reads the list.
    """
    if status not in FAILED or options.driver is not None:
        return status
    held = quarantined(options.flake, options.check)
    if held is None:
        return status
    if not excused(held, failed):
        unlisted = ", ".join(sorted(failed - (listed_subtests(held) or frozenset())))
        print(
            f"run-subtests: {options.check} failed in {unlisted or 'no test its log names'}, "
            "which main's quarantine doesn't excuse, so it blocks",
            file=sys.stderr,
        )
        return status
    reason = held.get("reason") or "no reason given"
    owner = f"; #{held['owner_pr']} owns it" if held.get("owner_pr") else ""
    print(
        f"run-subtests: {options.check} failed, but main quarantines it "
        f"({reason}{owner}), so it blocks nothing",
        file=sys.stderr,
    )
    return 0


def main(argv: Sequence[str]) -> int:
    """List a check's subtests, or run those the words name."""
    options = arguments(argv)
    if options.there:
        built = there(options)
        if built is not None:
            return blocking(options, *built)
        print("nix-offload can't build it elsewhere, so it runs here", file=sys.stderr)
    # A run here shows its output as it comes and reads none of it, so it
    # names no failed test.
    return blocking(options, ran(options), frozenset())


def ran(options: Options) -> int:
    """List a check's subtests, or run those the words name, here, and give the status."""
    driver = options.driver or build_driver(options.flake, options.check)
    script = (driver / "test-script").read_text()
    if pytest_driven(script):
        return run_pytest(driver, script, options)
    if not options.words:
        listed = "\n".join(names(script))
        print(listed)
        if options.out:
            options.out.mkdir(parents=True, exist_ok=True)
            (options.out / "subtests").write_text(listed + "\n")
        return 0
    chosen = [name for name in names(script) if named(name, options.words)]
    if not chosen:
        print(f"no subtest of {options.check} is named by {options.words}", file=sys.stderr)
        return USAGE
    print("running:\n  " + "\n  ".join(chosen), file=sys.stderr)
    out = options.out or Path(tempfile.mkdtemp(prefix=f"{options.check}."))
    out.mkdir(parents=True, exist_ok=True)
    status = run(driver, cut(script, options.words), out, options)
    say_pictures(out, options.check)
    return status


def run_pytest(driver: Path, script: str, options: Options) -> int:
    """List a pytest-driven check's tests, or run those the words name.

    A selection naming no test fails the run, as vm_pytest makes it.
    """
    if not options.words:
        out = Path(tempfile.mkdtemp(prefix=f"{options.check}."))
        try:
            return run(driver, script, out, options, "--collect-only --quiet")
        finally:
            shutil.rmtree(out, ignore_errors=True)
    try:
        chosen = selecting(options.words)
    except ValueError as error:
        print(f"run-subtests: {error}", file=sys.stderr)
        return USAGE
    print(f"running the tests pytest picks with {chosen}", file=sys.stderr)
    out = options.out or Path(tempfile.mkdtemp(prefix=f"{options.check}."))
    out.mkdir(parents=True, exist_ok=True)
    status = run(driver, script, out, options, chosen)
    print(f"pictures: {out}", file=sys.stderr)
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
