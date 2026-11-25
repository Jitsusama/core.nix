"""Run some of a VM test's subtests, in seconds rather than minutes.

A VM test's driver builds in seconds once its machines are built. Every VM
test hands its tests to pytest inside the driver (tests/lib/vm_pytest.py),
so this builds the check's driver and runs its script in a runtime folder of
its own, so two runs at once don't share the driver's network switch or
machine state, with pytest told which tests to pick.

    run-subtests [options] <check> [<words>...]

Each test whose name holds any of the words runs, a word's spaces read as
underscores, after only the fixtures it asks for. With none, pytest lists
the tests, which boots nothing, and it says it ran none, since it still exits
0. The pictures are kept in the folder it prints at the end. A check whose
script doesn't hand its tests to pytest is refused, as having none to pick.

With --there, all of it is built elsewhere: nix-offload builds the flake's
`subtests` for the check and words where the VM can run (subtests.nix),
which runs this same program on the check's driver there, and copies the
pictures back. An nix-offload that can't take that runs it here.

A check the net holds in quarantine on origin's main
(tools/merge-net/quarantine.json) still runs, but its failure is said and
blocks nothing: the run exits 0. An entry naming subtests excuses only a run
built elsewhere whose every failed subtest it names, as the net judges one.
A test may have several entries, each owned by the PR that broke what it
lists, and they are read together, each failure said with its own owner.
"""

import argparse
import ast
import contextlib
import json
import os
import platform
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
# subtests.rs reads it: by its function's name, its parameters dropped,
# FAILED or, for one that erred in a fixture, ERROR, and only at the start
# of pytest's own line, past what Nix puts before it, so a guest's echoed
# FAILED names none.
FAILED_TEST = re.compile(r"(?:\s*> |[^ ]+?> )?(?:FAILED|ERROR) \S*::(test_\w+)")

# A test pytest collects as its quiet listing says it, by its file and,
# past the first `::`, its name, parameters and all, which may hold a space,
# as `test_typed[the password]` does.
COLLECTED = re.compile(r"\S+?\.py::(?P<test>\S.*)")
# What pytest is told to list a check's tests with, booting nothing.
LISTING = "--collect-only --quiet"


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
        # Found on the path, as said above, and run with the person's own words.
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
    build = os.environ.get("TMPDIR", "")
    # mkdtemp makes a new folder only its owner can enter, so /tmp is safe.
    # A Nix build has no runtime folder and maybe no /tmp, but a short TMPDIR.
    fallback = build if 0 < len(build) <= LONGEST_RUNTIME_BASE else "/tmp"  # noqa: S108
    folder = Path(tempfile.mkdtemp(prefix="vm.", dir=base if short else fallback))
    try:
        yield folder
    finally:
        shutil.rmtree(folder, ignore_errors=True)


def run(driver: Path, script: str, out: Path, options: Options, pytest: str = "") -> int:
    """Run the script on the driver, its pictures kept in out, pytest given those options.

    A listing given a folder, as the build for --there is, keeps the names of
    the tests pytest collects there too, for the run that asked.
    """
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
        if pytest != LISTING or options.out is None:
            # The driver is the one the check built, given the person's own words.
            return subprocess.run(  # noqa: S603
                command,
                env=environment,
                check=False,
            ).returncode
        # Echoed as it comes, since the build's log is the listing's only
        # other copy, and kept only from a listing that passed, so a list
        # left from an earlier one is never taken for this one's.
        listed = out / "subtests"
        listed.unlink(missing_ok=True)
        names: list[str] = []
        # The same driver, run the same way, with its output read for the names.
        with subprocess.Popen(  # noqa: S603
            command, env=environment, stdout=subprocess.PIPE, text=True, errors="replace"
        ) as running:
            for line in running.stdout or ():
                sys.stdout.write(line)
                if (found := COLLECTED.fullmatch(line.strip())) is not None:
                    names.append(found["test"])
        if running.returncode == 0:
            listed.write_text("".join(f"{name}\n" for name in names))
        return running.returncode


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
        found.group(1) for line in lines if (found := FAILED_TEST.match(line)) is not None
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
    # A list left in a folder used before isn't this build's.
    listed = out / "subtests"
    listed.unlink(missing_ok=True)
    command = ["nix-offload", "there", "-L", "--out", str(out), "--apply", apply]
    print(f"building {options.check}'s subtests where nix-offload builds", file=sys.stderr)
    try:
        status, refused, failed = offloaded([*command, flake, f"legacyPackages.{system}.subtests"])
    except FileNotFoundError:
        return None
    if refused:
        print(
            f"run-subtests: {options.check} isn't a VM test; build it whole with {whole(options)}",
            file=sys.stderr,
        )
        return USAGE, frozenset()
    if status == USAGE:
        return None
    # A listing takes no pictures; the build keeps the names it collected, so
    # one served from what's built already lists them too.
    if options.words:
        say_pictures(out, options.check)
    elif listed.exists():
        print(listed.read_text(), end="")
    return status, failed


def offloaded(command: list[str]) -> tuple[int, bool, frozenset[str]]:
    """Run nix-offload, and give its status, whether it refused the check, and what failed.

    Its errors are shown as they come, and read for subtests.nix's refusal
    of a check that isn't a VM test and for the tests the build says failed.
    """
    refused = False
    said: list[str] = []
    # The command is nix-offload's, made here from the check and the person's words.
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


def quarantined(flake: str, check: str) -> list[dict[str, object]]:
    """Give origin main's quarantine entries for the check, none when it holds none.

    A gallery, `vm-a.gallery`, is held with its test. A repository without
    the list, or without git, quarantines nothing, and a list that doesn't
    read is said so and quarantines nothing either.
    """
    folder = "." if ":" in flake else flake
    try:
        # git is the person's own, found on the path, as nix is.
        shown = subprocess.run(  # noqa: S603
            # Reads main's quarantine list and nothing else.
            ["git", "-C", folder, "show", f"{MAIN}:{QUARANTINE}"],  # noqa: S607
            check=False,
            capture_output=True,
            text=True,
        )
    except FileNotFoundError:
        return []
    if shown.returncode != 0:
        return []
    try:
        entries: object = json.loads(shown.stdout)
    except json.JSONDecodeError as error:
        print(f"run-subtests: {QUARANTINE} on main doesn't read: {error}", file=sys.stderr)
        return []
    test = check.split(".", maxsplit=1)[0]
    return [
        cast("dict[str, object]", entry)
        for entry in (entries if isinstance(entries, list) else [])
        if isinstance(entry, dict) and entry.get("test") == test
    ]


def excused(held: list[dict[str, object]], failed: frozenset[str]) -> bool:
    """Say whether a test's quarantine entries excuse a run in which those tests failed.

    A whole entry excuses any failure. Entries naming subtests excuse a run
    only when they name every test that failed between them, and never a
    run that names none, such as one that timed out or ran here, as
    merge-net judges it.
    """
    if any(listed_subtests(entry) is None for entry in held):
        return True
    return bool(failed) and not unlisted(held, failed)


def unlisted(held: list[dict[str, object]], failed: frozenset[str]) -> frozenset[str]:
    """Give the tests that failed which no entry names."""
    left = failed
    for entry in held:
        left -= listed_subtests(entry) or frozenset()
    return left


def listed_subtests(held: dict[str, object]) -> frozenset[str] | None:
    """Give the subtests a quarantine entry names, or None for a whole entry."""
    listed = held.get("subtests")
    if not isinstance(listed, list):
        return None
    return frozenset(name for name in cast("list[object]", listed) if isinstance(name, str))


def said(held: list[dict[str, object]], failed: frozenset[str]) -> str:
    """Say why a failure in those tests blocks nothing, with each one's owner.

    A whole entry is said alone; otherwise each entry holding a test that
    failed is said with those tests, so each owner reads what is theirs.
    """

    def why(entry: dict[str, object]) -> str:
        reason = entry.get("reason") or "no reason given"
        owner = f"; #{entry['owner_pr']} owns it" if entry.get("owner_pr") else ""
        return f"({reason}{owner})"

    for entry in held:
        if listed_subtests(entry) is None:
            return f"main quarantines it {why(entry)}"
    each = [
        f"{', '.join(sorted(met))} {why(entry)}"
        for entry in held
        if (met := failed & (listed_subtests(entry) or frozenset()))
    ]
    return f"main quarantines {' and '.join(each)}"


def blocking(options: Options, status: int, failed: frozenset[str]) -> int:
    """Give the run's status, a failure main's quarantine excuses read as none.

    Inside a build, given its driver, the status is the build's own, and
    the run that sent it there reads the list.
    """
    if status not in FAILED or options.driver is not None:
        return status
    held = quarantined(options.flake, options.check)
    if not held:
        return status
    if not excused(held, failed):
        named = ", ".join(sorted(unlisted(held, failed)))
        print(
            f"run-subtests: {options.check} failed in {named or 'no test its log names'}, "
            "which main's quarantine doesn't excuse, so it blocks",
            file=sys.stderr,
        )
        return status
    print(
        f"run-subtests: {options.check} failed, but {said(held, failed)}, so it blocks nothing",
        file=sys.stderr,
    )
    return 0


def main(argv: Sequence[str]) -> int:
    """List a check's subtests, or run those the words name."""
    options = arguments(argv)
    status = listed_or_ran(options)
    # A listing exits 0 having run nothing, so one asked for as a run would
    # pass unseen: on 15 November a session ran fifteen tests so. The build
    # a --there run makes lists with a driver given, and says nothing.
    if not options.words and status == 0 and options.driver is None:
        print(ran_none(options), file=sys.stderr)
    return status


def ran_none(options: Options) -> str:
    """Say that a listing ran no subtest, and how to run some or all."""
    return (
        f"run-subtests: listed {options.check}'s subtests and ran none; name words from them "
        f"to run those, or build the whole test with {whole(options)}"
    )


def whole(options: Options) -> str:
    """Give the command that builds the whole check from its flake, for this machine's system."""
    machine = {"arm64": "aarch64"}.get(platform.machine(), platform.machine())
    system = f"{machine}-{platform.system().lower()}"
    return f"nix-offload there {options.flake} checks.{system}.{options.check}"


def listed_or_ran(options: Options) -> int:
    """List a check's subtests, or run those the words name, here or elsewhere."""
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
    if not pytest_driven(script):
        print(
            f"run-subtests: {options.check}'s script doesn't hand its tests to pytest "
            f"(tests/lib/vm_pytest.py), so it has none to pick; build it whole with "
            f"{whole(options)}",
            file=sys.stderr,
        )
        return USAGE
    return run_pytest(driver, script, options)


def run_pytest(driver: Path, script: str, options: Options) -> int:
    """List a check's tests, or run those the words name.

    A selection naming no test fails the run, as vm_pytest makes it.
    """
    if not options.words and options.out is not None:
        options.out.mkdir(parents=True, exist_ok=True)
        return run(driver, script, options.out, options, LISTING)
    if not options.words:
        out = Path(tempfile.mkdtemp(prefix=f"{options.check}."))
        try:
            return run(driver, script, out, options, LISTING)
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
