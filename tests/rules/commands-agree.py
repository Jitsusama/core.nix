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

A script is a command too: a program a Nix file outside the tests builds
with writeShellApplication or writeShellScriptBin, named by the wrapper, or
by the file it reads when its name is worked out. Its text is the files its
wrapper's text reads and the strings it spells. It keeps the convention by
hand, since the kit is Rust, and agrees when:

- exits: every exit it spells with a number is one of the table's;
- inks: it spells no escape sequence and sets no colour with tput;
- helps: run with `--help`, it opens its answer on stdout with its own name,
  says nothing on stderr and exits 0, within a few seconds.

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
from dataclasses import dataclass, replace
from pathlib import Path

POINTS = ("kit", "exits", "inks", "helps")

# The points a script is held to: the kit is a Rust crate, so it keeps the
# rest by hand.
SCRIPT_POINTS = ("exits", "inks", "helps")

# What each point asks of a command that breaks it.
FIXES = {
    "kit": "build it on modules/terminal-kit, through terminal_kit::main",
    "exits": "end it by returning from terminal_kit::main, with a Failure's Exit",
    "inks": "draw it with the kit's Pen, which reads the theme's roles and marks",
    "helps": "declare its help with terminal_kit::Help and give the check its package",
}

# What each point asks of a script that breaks it.
SCRIPT_FIXES = {
    "exits": "exit with a code from docs/commands.md's table",
    "inks": "leave colour to a command on the kit, or draw none",
    "helps": "answer --help with its name first, and give the check its package",
}

# Every code docs/commands.md's table gives a meaning.
TABLE = frozenset({0, 1, 64, 65, 66, 69, 70, 75, 77, 78, 130})

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

# The two wrappers that make a script a command, and what they're told.
APPLICATION = re.compile(r"\bwriteShellApplication\s*\{")
SCRIPT_BIN = re.compile(r'\bwriteShellScriptBin\s+"([^"$]+)"\s*')
NAME = re.compile(r'\bname\s*=\s*"([^"$]+)"\s*;')
TEXT = re.compile(r"\btext\s*=\s*")
READS = re.compile(r"\breadFile\s+(\.{1,2}/[\w./-]+)")
# A script a wrapper runs by its path rather than reading it in.
RUNS = re.compile(r"\$\{(\.{1,2}/[\w./-]+\.sh)\}")

# What a script ends with, and how it would ink, by its usual spellings.
SCRIPT_EXIT = re.compile(r"\bexit\s+(\d+)")
SCRIPT_INKS = re.compile(r"\\x1[bB]\[|\\u001[bB]\[|\\033\[|\\e\[|\btput\s+set")


@dataclass(frozen=True)
class Command:
    """One command: a crate's program, from its manifest, or a script, from its text."""

    name: str
    manifest: Path
    text: str | None = None
    # A script by a crate program's own name, which tells it its settings,
    # is that program as it's typed, so its points are the program's too.
    wrapper: str | None = None

    @property
    def is_script(self) -> bool:
        """Say whether a Nix wrapper builds it from a script rather than a crate."""
        return self.text is not None


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


def indented(text: str, start: int) -> str | None:
    """Give the Nix indented string opening at start, or None when none opens there."""
    if not text.startswith("''", start):
        return None
    at = start + 2
    while (end := text.find("''", at)) != -1:
        # ''' , ''$ and ''\ are the string's escapes, not its end.
        if text[end + 2 : end + 3] in {"'", "$", "\\"}:
            at = end + 3
            continue
        return text[start + 2 : end]
    return None


def spelled(nix: Path, expression: str) -> tuple[str, list[Path]]:
    """Give the text a wrapper's expression spells, from its strings and the files it reads."""
    parts, read = [], []
    at = 0
    while (start := expression.find("''", at)) != -1:
        string = indented(expression, start)
        if string is None:
            break
        parts.append(string)
        at = start + len(string) + 4
    for relative in READS.findall(expression) + RUNS.findall(expression):
        path = (nix.parent / relative).resolve()
        if path.is_file() and path.suffix != ".nix":
            parts.append(path.read_text())
            read.append(path)
    return "\n".join(parts), read


def closing(text: str, start: int) -> int:
    """Give where the attribute set opened just before start closes."""
    depth, at = 1, start
    while depth and at < len(text):
        depth += {"{": 1, "}": -1}.get(text[at], 0)
        at += 1
    return at


def scripts(tree: Path) -> list[Command]:
    """Give each script a Nix file outside the tests wraps as a program, with its text."""
    found: dict[str, Command] = {}
    for nix in sorted(tree.rglob("*.nix")):
        relative = nix.relative_to(tree)
        if {".git", "tests", "checks"} & set(relative.parts) or nix.name.startswith("checks"):
            continue
        text = nix.read_text()
        for match in SCRIPT_BIN.finditer(text):
            # The text is the one argument after the name: a string or a read file.
            rest = text[match.end() :]
            string = indented(rest, 0)
            argument = rest[: len(string) + 4] if string is not None else rest.split("\n", 1)[0]
            body, _ = spelled(nix, argument)
            found.setdefault(match.group(1), Command(match.group(1), nix, body))
        for match in APPLICATION.finditer(text):
            block = text[match.end() : closing(text, match.end())]
            said = TEXT.search(block)
            body, read = spelled(nix, block[said.end() :] if said else "")
            named = NAME.search(block)
            name = named.group(1) if named else read[0].stem if read else None
            if name:
                found.setdefault(name, Command(name, nix, body))
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


def script_helps(name: str, program: str) -> list[str]:
    """Say how a script's help falls short, if it does."""
    said = ran(program, "--help")
    if said is None:
        return [f"--help took over {HELP_SECONDS} seconds, so it started something"]
    wrong = []
    if said.returncode != 0:
        wrong.append(f"--help exited {said.returncode}, not 0")
    if said.stderr:
        wrong.append(f"--help said {said.stderr.strip()[:80]!r} on stderr")
    if not said.stdout.lstrip().startswith(name):
        wrong.append("--help didn't open with its name")
    return wrong


def spelled_breaks(text: str, whose: str) -> dict[str, list[str]]:
    """Give the exits and inks a script's text breaks, past its comments."""
    code = "\n".join(line for line in text.splitlines() if not line.lstrip().startswith("#"))
    out: dict[str, list[str]] = {}
    codes = sorted({int(code) for code in SCRIPT_EXIT.findall(code)} - TABLE)
    if codes:
        out["exits"] = [f"{whose} exits {', '.join(map(str, codes))}"]
    if SCRIPT_INKS.search(code):
        out["inks"] = [f"{whose} spells an escape sequence"]
    return out


def script_broken(command: Command, runs: dict[str, str]) -> dict[str, list[str]]:
    """Give the points a script breaks, each with what was seen."""
    out = spelled_breaks(command.text or "", "it")
    if command.name in runs:
        wrong = script_helps(command.name, runs[command.name])
        if wrong:
            out["helps"] = wrong
    else:
        out["helps"] = ["the check isn't given it to run"]
    return out


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
    if command.is_script:
        return script_broken(command, runs)
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
    if command.wrapper is not None:
        for point, seen in spelled_breaks(command.wrapper, "its wrapper").items():
            out.setdefault(point, []).extend(seen)
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
        where = (command.manifest if command.is_script else command.manifest.parent).relative_to(
            tree
        )
        for point in sorted(set(breaks) - waits, key=POINTS.index):
            seen = "".join(f"; {words}" for words in breaks[point])
            wrapper = all(words.startswith("its wrapper") for words in breaks[point])
            fix = (SCRIPT_FIXES if command.is_script or wrapper else FIXES)[point]
            out.append(f"{command.name} ({where}) breaks {point}{seen}: {fix}.")
        if command.is_script and "kit" in waits:
            out.append(f"{command.name} is a script, which waits on no kit: take it off.")
        out.extend(
            f"{command.name} keeps {point} now: take it off the waiting list."
            for point in sorted(waits - set(breaks), key=POINTS.index)
        )
        if not breaks:
            agree += 1
    out.extend(
        f"{name} waits, but nothing builds it: take it off the waiting list."
        for name in sorted(set(waiting) - names)
    )
    out.extend(
        f"{name} is given to run, but nothing builds it." for name in sorted(set(runs) - names)
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
    wrapped = {script.name: script for script in scripts(tree)}
    crates = [
        replace(crate, wrapper=wrapped.pop(crate.name).text) if crate.name in wrapped else crate
        for crate in commands(tree)
    ]
    found = sorted([*crates, *wrapped.values()], key=lambda command: command.name)
    out, agree = judged(found, runs, waiting, tree)
    for line in out:
        print(line)
    print(f"{agree} of {len(found)} commands agree with docs/commands.md.")
    return 1 if out else 0


if __name__ == "__main__":
    sys.exit(main())
