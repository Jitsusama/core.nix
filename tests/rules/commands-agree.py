"""Check that every command this repository builds keeps the convention docs/commands.md sets.

    commands-agree TREE RUNS WAITING

A command here is a program built from one of TREE's crates, by the crate's
src/main.rs, its src/bin or a [[bin]] it declares. It agrees when:

- kit: its crate builds on the terminal kit, which gives it the shared
  flags, the ink decision, help, the error's three parts and the exit codes;
- exits: its source never ends the process itself, with process::exit or an
  ExitCode made from a number, so the codes are the table's, set by the kit;
- inks: its source spells no escape sequence and takes no colour crate, so
  its colour follows --color, NO_COLOR, FORCE_COLOR and the theme;
- helps: run with `--help --format json`, it answers help/1 under its own
  name on stdout, says nothing on stderr and exits 0, within a few seconds
  and so without starting anything; and `--version` answers one line.

RUNS is JSON naming, for each command the check is given built, the program
to run; a command on the kit has to be given, since its help is only proven
by running it. WAITING is JSON naming, for each command that broke the
convention before the rule, the points it breaks while it comes to agree. A
command breaking a point it doesn't wait on fails, as does one waiting on a
point it now keeps, so the list only shrinks. Every failure is printed before
the check exits 1, and the last line says how many agree.
"""

import json
import os
import re
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import dataclass
from pathlib import Path

POINTS = ("kit", "exits", "inks", "helps")

# What each point asks of a command that breaks it.
FIXES = {
    "kit": "build it on modules/terminal-kit, through terminal_kit::main",
    "exits": "end it by returning from terminal_kit::main, with a Failure's Exit",
    "inks": "draw it with the kit's Pen, which reads the theme's roles and marks",
    "helps": "declare its help with terminal_kit::Help and give the check its package",
}

# How long a command may take to answer its help: one that takes longer is
# doing something, which help never starts.
HELP_SECONDS = 10

# An exit the source makes itself rather than returning a Failure to the kit.
EXITS = re.compile(r"\bprocess::exit\s*\(|\bExitCode::from\s*\(")

# An escape sequence spelled in Rust source, by any of its usual spellings.
ESCAPES = re.compile(r"\\x1[bB]\[|\\u\{1[bB]\}\[|\\033\[|\\e\[")

# Crates that draw colour their own way, around the kit's decision.
COLOUR_CRATES = ("anstyle", "ansi_term", "colored", "console", "owo-colors", "termcolor", "yansi")

# Where a Rust file's unit tests start, which hold what a test expects to see.
TESTS = re.compile(r"^#\[cfg\(test\)\]", re.MULTILINE)


@dataclass(frozen=True)
class Command:
    """One command, as its crate builds it."""

    name: str
    manifest: Path


def commands(tree: Path) -> list[Command]:
    """Give each program TREE's crates build, with the crate it's built from."""
    found: dict[str, Command] = {}
    for manifest in sorted(tree.rglob("Cargo.toml")):
        relative = manifest.relative_to(tree)
        if ".git" in relative.parts or "target" in relative.parts:
            continue
        crate = tomllib.loads(manifest.read_text())
        package = crate.get("package", {}).get("name")
        source = manifest.parent / "src"
        names = [binary["name"] for binary in crate.get("bin", []) if "name" in binary]
        if package and (source / "main.rs").is_file():
            names.append(package)
        if (source / "bin").is_dir():
            names += [path.stem for path in (source / "bin").glob("*.rs")]
            names += [path.parent.name for path in (source / "bin").glob("*/main.rs")]
        for name in names:
            found.setdefault(name, Command(name, manifest))
    return sorted(found.values(), key=lambda command: command.name)


def dependencies(manifest: Path) -> set[str]:
    """Give the crates a crate depends on, for building or for its tests alike."""
    crate = tomllib.loads(manifest.read_text())
    return set(crate.get("dependencies", {}))


def source(manifest: Path) -> str:
    """Give a crate's own code, without its tests, which may spell what they expect."""
    parts = []
    for path in sorted((manifest.parent / "src").rglob("*.rs")):
        if path.name == "tests.rs" or "tests" in path.relative_to(manifest.parent).parts:
            continue
        parts.append(TESTS.split(path.read_text(), maxsplit=1)[0])
    return "\n".join(parts)


def ran(program: str, *args: str) -> subprocess.CompletedProcess[str] | None:
    """Run a command as a person's shell would, with nothing of theirs to start."""
    with tempfile.TemporaryDirectory() as home:
        environment = {"HOME": home, "PATH": os.environ.get("PATH", ""), "TERM": "dumb"}
        try:
            # The program is one the check was given built, from Nix.
            return subprocess.run(  # noqa: S603
                [program, *args],
                capture_output=True,
                text=True,
                env=environment,
                timeout=HELP_SECONDS,
                check=False,
                stdin=subprocess.DEVNULL,
            )
        except subprocess.TimeoutExpired:
            return None


def helps(name: str, program: str) -> list[str]:
    """Say how a command's help and version fall short, if they do."""
    said = ran(program, "--help", "--format", "json")
    if said is None:
        return [f"--help took over {HELP_SECONDS} seconds, so it started something"]
    wrong = []
    if said.returncode != 0:
        wrong.append(f"--help exited {said.returncode}, not 0")
    if said.stderr:
        wrong.append(f"--help said {said.stderr.strip()[:80]!r} on stderr")
    try:
        answer = json.loads(said.stdout)
    except json.JSONDecodeError:
        answer = None
    if not isinstance(answer, dict) or answer.get("schema") != "help/1":
        wrong.append("--help --format json didn't answer help/1")
    elif answer.get("name") != name:
        wrong.append(f"--help names it {answer.get('name')!r}")
    version = ran(program, "--version")
    if version is None or version.returncode != 0 or len(version.stdout.splitlines()) != 1:
        wrong.append("--version didn't answer one line")
    return wrong


def broken(command: Command, runs: dict[str, str]) -> dict[str, list[str]]:
    """Give the points a command breaks, each with what was seen."""
    code = source(command.manifest)
    used = dependencies(command.manifest)
    out: dict[str, list[str]] = {}
    if "terminal-kit" not in used:
        out["kit"] = []
    if EXITS.search(code):
        out["exits"] = ["its source ends the process itself"]
    colours = sorted(used & set(COLOUR_CRATES))
    if ESCAPES.search(code) or colours:
        out["inks"] = [f"it takes {', '.join(colours)}"] if colours else []
    if command.name in runs:
        wrong = helps(command.name, runs[command.name])
        if wrong:
            out["helps"] = wrong
    else:
        out["helps"] = ["the check isn't given it to run"] if "kit" not in out else []
    return out


def judged(
    found: list[Command], runs: dict[str, str], waiting: dict[str, list[str]], tree: Path
) -> tuple[list[str], int]:
    """Say every way the commands disagree with the convention, and how many agree."""
    out = []
    agree = 0
    names = {command.name for command in found}
    for command in found:
        breaks = broken(command, runs)
        waits = set(waiting.get(command.name, []))
        where = command.manifest.parent.relative_to(tree)
        for point in sorted(set(breaks) - waits, key=POINTS.index):
            seen = "".join(f"; {words}" for words in breaks[point])
            out.append(f"{command.name} ({where}) breaks {point}{seen}: {FIXES[point]}.")
        out.extend(
            f"{command.name} keeps {point} now: take it off the waiting list."
            for point in sorted(waits - set(breaks), key=POINTS.index)
        )
        if not breaks:
            agree += 1
    out.extend(
        f"{name} waits, but no crate builds it: take it off the waiting list."
        for name in sorted(set(waiting) - names)
    )
    out.extend(
        f"{name} is given to run, but no crate builds it." for name in sorted(set(runs) - names)
    )
    for name, points in sorted(waiting.items()):
        unknown = sorted(set(points) - set(POINTS))
        if unknown:
            out.append(f"{name} waits on {', '.join(unknown)}, which no point is called.")
    return out, agree


def main() -> int:
    """Check the tree's commands, printing each disagreement."""
    tree, runs_file, waiting_file = (Path(arg) for arg in sys.argv[1:4])
    runs = json.loads(runs_file.read_text())
    waiting = json.loads(waiting_file.read_text())
    found = commands(tree)
    out, agree = judged(found, runs, waiting, tree)
    for line in out:
        print(line)
    print(f"{agree} of {len(found)} commands agree with docs/commands.md.")
    return 1 if out else 0


if __name__ == "__main__":
    sys.exit(main())
