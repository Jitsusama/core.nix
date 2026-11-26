"""Tell a crate's own Rust code from what only its tests read, as rustc finds its modules.

A file is read only by tests when a module declared under cfgs that hold only
under test, such as `#[cfg(test)]` or `#[cfg(all(feature = "x", test))]`, or
inside an inline module declared so, is read from it, or a module such a file
declares is, and no module of the crate's own code is read from it too. A
module inside inline ones is looked for in their folders, as rustc looks. A
file's name says nothing either way: a module named `x_tests` may be real
code, and one named `golden` read by tests alone. A file no module declares
isn't built, and is taken as the crate's own code, since nothing says else.

Modules and their braces are read from a file's text with what its comments,
strings and character literals hold blanked, as rustc's lexer passes over
them, so a fixture's `}` or a doc line's `mod` is neither.
"""

import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Iterator

# A module declared, with the attributes and doc lines above it, and whether
# its body follows inline. An attribute may run over lines, as rustfmt wraps
# a long one, share one with another or with the module, and be parted from
# the module by blank lines and comments, as rustc allows.
MODULE = re.compile(
    r"(?m)^[ \t]*((?:(?:#\[[^\]]*\](?:\s*//[^\n]*\n)*\s*|///[^\n]*\n)[ \t]*)*)"
    r"(?:pub\S* )?mod (\w+)[ \t]*([;{])"
)

# A cfg among a module's attributes, and what it asks.
CFG = re.compile(r"#\[\s*cfg\s*\((.*?)\)\s*\]", re.DOTALL)

# A cfg's words, strings and punctuation.
TOKEN = re.compile(r'"(?:\\.|[^"\\])*"|\w+|[(),=]')

# Where an attribute says a module is read from, in place of its name.
PATH = re.compile(r'#\[\s*path\s*=\s*"([^"]+)"\s*\]')

# What starts a comment, a string or a character literal, or a word, which
# is read whole so that an `r` or a `b` inside one starts nothing.
LEXEME = re.compile(
    r"""(?P<raw>[bc]?r(?P<hashes>\#*)")"""
    r"""|(?P<quote>[bc]?")"""
    r"""|(?P<char>'(?:\\(?:u\{[0-9a-fA-F_]*\}|x[0-9a-fA-F]{2}|.)|[^\\'\n])')"""
    r"""|(?P<line>//+)"""
    r"""|(?P<block>/\*)"""
    r"""|\w+"""
)

# The rest of a string after its opening quote, through its closing one.
STRING_REST = re.compile(r'(?:\\.|[^"\\])*"', re.DOTALL)

# Where a block comment opens or closes, since they nest.
BLOCK_EDGE = re.compile(r"/\*|\*/")


type Predicate = tuple[str, list["Predicate"] | None]


class RefusedError(ValueError):
    """A crate the reader won't read rather than guess at, and why."""

    def within(self, path: Path) -> RefusedError:
        """Give the same refusal, said of the file `path` it was met in."""
        return RefusedError(f"{path}: {self}")


def blank(text: str) -> str:
    """Give text with every character but its newlines a space."""
    return re.sub(r"[^\n]", " ", text)


def block_comment_end(text: str, at: int) -> int:
    """Give where the block comment whose `/*` ends at `at` closes."""
    depth = 1
    for edge in BLOCK_EDGE.finditer(text, at):
        depth += 1 if edge.group() == "/*" else -1
        if not depth:
            return edge.end()
    why = "a block comment never closes"
    raise RefusedError(why)


def masked(text: str) -> str:
    """Give a file's text with what its comments, strings and character literals hold blanked.

    Each keeps its place and its newlines, a string and a character literal
    their quotes, and a line comment its slashes, so a doc line is still one.
    """
    kept: list[str] = []
    at = 0
    while found := LEXEME.search(text, at):
        kept.append(text[at : found.start()])
        start, opened = found.start(), found.end()
        if found["raw"]:
            close = text.find('"' + found["hashes"], opened)
            if close < 0:
                why = "a raw string never closes"
                raise RefusedError(why)
            end = close + 1 + len(found["hashes"])
            kept.append(found.group() + blank(text[opened:close]) + text[close:end])
        elif found["quote"]:
            rest = STRING_REST.match(text, opened)
            if rest is None:
                why = "a string never closes"
                raise RefusedError(why)
            end = rest.end()
            kept.append(found.group() + blank(text[opened : end - 1]) + '"')
        elif found["char"]:
            end = opened
            kept.append("'" + blank(text[start + 1 : end - 1]) + "'")
        elif found["line"]:
            close = text.find("\n", opened)
            end = len(text) if close < 0 else close
            kept.append(found.group() + blank(text[opened:end]))
        elif found["block"]:
            end = block_comment_end(text, opened)
            kept.append(blank(text[start:end]))
        else:
            end = opened
            kept.append(found.group())
        at = end
    kept.append(text[at:])
    return "".join(kept)


def predicate(tokens: list[str], at: int = 0) -> tuple[Predicate, int]:
    """Read the cfg predicate that starts at `at`, and where it ends."""
    name = tokens[at]
    at += 1
    if at < len(tokens) and tokens[at] == "=":
        return (f"{name}=", None), at + 2
    if at < len(tokens) and tokens[at] == "(":
        at += 1
        given: list[Predicate] = []
        while tokens[at] != ")":
            argument, at = predicate(tokens, at)
            given.append(argument)
            if tokens[at] == ",":
                at += 1
        return (name, given), at + 1
    return (name, None), at


def read_cfg(asked: str, written: str) -> Predicate:
    """Read what a cfg asks, refusing one whose words don't make one predicate.

    It's read masked, as `asked`, and a refusal quotes it as `written`.
    """
    tokens = TOKEN.findall(asked)
    try:
        read, end = predicate(tokens)
    except IndexError:
        end = -1
    if end != len(tokens):
        why = f"a cfg the reader can't read: cfg({written})"
        raise RefusedError(why)
    return read


def cfgs(module: re.Match[str], text: str) -> list[Predicate]:
    """Read the cfgs among a module's attributes, found in its file's masked text.

    The attributes are read masked, so a doc line that mentions a cfg asks
    nothing, and each cfg is quoted from `text`, as written, if refused.
    """
    start = module.start(1)
    return [
        read_cfg(cfg.group(1), text[start + cfg.start(1) : start + cfg.end(1)])
        for cfg in CFG.finditer(module.group(1))
    ]


def paths_given(
    text: str, blanked: str, start: int = 0, end: int | None = None
) -> Iterator[re.Match[str]]:
    """Give each path attribute in `text` between `start` and `end`.

    Each is read from `text`, since masking blanks the path, but only where
    its `#` is still there masked, so one in a doc line or a string is none.
    """
    found = PATH.finditer(text, start, len(text) if end is None else end)
    return (given for given in found if blanked[given.start()] == "#")


def path_given(module: re.Match[str], text: str, blanked: str) -> re.Match[str] | None:
    """Give the path attribute among a module's attributes, if one is there."""
    return next(paths_given(text, blanked, module.start(1), module.end(1)), None)


def holds_only_under_test(asked: Predicate) -> bool:
    """Whether a cfg predicate can hold only where tests are built."""
    name, given = asked
    if given is None:
        return name == "test"
    if name == "all":
        return any(holds_only_under_test(argument) for argument in given)
    if name == "any":
        return bool(given) and all(holds_only_under_test(argument) for argument in given)
    # `not(test)` holds outside tests, and nothing else names them.
    return False


def holds_wherever_tests_are_built(asked: Predicate) -> bool:
    """Whether a cfg predicate holds in every build of the tests, whatever else is set."""
    name, given = asked
    if given is None:
        return name == "test"
    if name == "all":
        return all(holds_wherever_tests_are_built(argument) for argument in given)
    if name == "any":
        return any(holds_wherever_tests_are_built(argument) for argument in given)
    return False


def only_under_test(asked: list[Predicate]) -> bool:
    """Whether the cfgs among a module's attributes hold only where tests are built.

    Several cfgs must all hold, so one that holds only under test is enough.
    """
    return any(holds_only_under_test(each) for each in asked)


def needs_its_file(asked: list[Predicate]) -> bool:
    """Whether a module read under test is built in every build of the tests.

    One whose cfg asks more than tests, such as a target, may be ruled out
    where its file isn't, so it needn't have one.
    """
    return all(holds_wherever_tests_are_built(each) for each in asked)


def read(path: Path) -> tuple[str, str]:
    """Give a file's text, and that text masked."""
    text = path.read_text()
    return text, masked(text)


@dataclass(frozen=True)
class Declared:
    """A module a file declares with a file of its own, as rustc finds it.

    `file` is its file, resolved, or None where it has none, `looked` where
    it was looked for, from the declaring file's folder, `given` whether a
    path attribute named it, `test` whether it's built only under test, and
    `needs` whether every build of the tests builds it, so it must have one.
    """

    name: str
    file: Path | None
    looked: str
    given: bool
    test: bool
    needs: bool


def declared(path: Path, here: Path, texts: tuple[str, str]) -> list[Declared]:
    """Give the modules with files of their own that the file at `path` declares.

    `here` is where its modules are found, and `texts` its text and that
    text masked. One inside inline modules is found in their folders, and
    asks their cfgs as well as its own.
    """
    text, blanked = texts
    modules = list(MODULE.finditer(blanked))
    inline = [
        (module, block_end(blanked, module.end())) for module in modules if module.group(3) == "{"
    ]
    out: list[Declared] = []
    for module in modules:
        if module.group(3) == "{":
            continue
        outer = [each for each, end in inline if each.end() <= module.start() < end]
        asked = [cfg for each in [*outer, module] for cfg in cfgs(each, text)]
        folder = here.joinpath(*(each.group(2) for each in outer))
        name = module.group(2)
        given = path_given(module, text, blanked)
        if given:
            # Inside an inline module, rustc reads a path from its folder.
            tried = [(folder if outer else path.parent) / given.group(1)]
        else:
            tried = [folder / f"{name}.rs", folder / name / "mod.rs"]
        file = next((each.resolve() for each in tried if each.exists()), None)
        looked = " or ".join(os.path.relpath(each, path.parent) for each in tried)
        test, needs = only_under_test(asked), needs_its_file(asked)
        out.append(Declared(name, file, looked, bool(given), test, needs))
    return out


def declarations(src: Path, beside: set[Path]) -> dict[Path, list[Declared]]:
    """Give what each file of the crate under `src` declares, and each file those name.

    A crate's roots, a mod.rs and a file named by a path attribute, one of
    `beside`, find their modules beside them; any other file finds them in
    the folder named as it is.
    """
    bins = (src / "bin").resolve()
    out: dict[Path, list[Declared]] = {}
    left = sorted(path.resolve() for path in src.rglob("*.rs"))
    while left:
        path = left.pop()
        if path in out:
            continue
        root = path.name in ("main.rs", "lib.rs", "mod.rs") or path.parent == bins
        here = path.parent if root or path in beside else path.with_suffix("")
        try:
            out[path] = declared(path, here, read(path))
        except RefusedError as error:
            raise error.within(path) from error
        left.extend(each.file for each in out[path] if each.file and each.file not in out)
    return out


def reached(starts: list[Path], through: dict[Path, list[Path]], past: set[Path]) -> set[Path]:
    """Give the files reached from `starts` through the files each names, none of `past`."""
    found: set[Path] = set()
    while starts:
        path = starts.pop()
        if path not in found and path not in past:
            found.add(path)
            starts.extend(through.get(path, []))
    return found


# The files under src cargo builds as crates of their own, by default.
CARGO_ROOTS = ("lib.rs", "main.rs", "bin/*.rs", "bin/*/main.rs")


def read_by_tests_alone(src: Path) -> set[Path]:
    """Give each Rust file under a crate's src that only its tests read, resolved.

    The crate's own code is read from its roots, the files cargo builds
    alone and each file no module declares, through the modules built
    outside tests, however many a test names by path too; what's read
    from it through modules built only under test, and on from there, and
    isn't its own code too, is the tests'. A module tests always build whose
    file is nowhere rustc would look is refused, naming where that is.
    """
    files = {path.resolve() for path in src.rglob("*.rs")}
    # Which files a path attribute names decides where theirs are found, so
    # the files are read again till no more are named so.
    beside: set[Path] = set()
    while True:
        known = declarations(src, beside)
        named = {
            each.file for found in known.values() for each in found if each.given and each.file
        }
        if named <= beside:
            break
        beside |= named
    declared_files = {each.file for found in known.values() for each in found}
    plainly = {
        path: [each.file for each in found if each.file and not each.test]
        for path, found in known.items()
    }
    everything = {path: [each.file for each in found if each.file] for path, found in known.items()}
    roots = {path.resolve() for pattern in CARGO_ROOTS for path in src.glob(pattern)}
    seeds = [path for path in sorted(known) if path not in declared_files or path in roots]
    own = reached(seeds, plainly, set())
    under_test = [each.file for path in own for each in known[path] if each.file and each.test]
    tests = reached(under_test, everything, own)
    for path in sorted(known):
        for each in known[path]:
            if each.file is None and each.needs and (each.test or path in tests):
                why = f"mod {each.name} is built under test with no file at {each.looked}"
                raise RefusedError(why).within(path)
    return tests & files


def block_end(blanked: str, at: int) -> int:
    """Give where the block whose `{` ends at `at` closes, past its `}`."""
    depth = 1
    for brace in re.finditer(r"[{}]", blanked[at:]):
        depth += 1 if brace.group() == "{" else -1
        if not depth:
            return at + brace.end()
    why = "an inline module's braces never close"
    raise RefusedError(why)


def without_inline_tests(text: str) -> str:
    """Give a file's text without the test modules written inline in it.

    Each runs to the `}` that closes its `{`, counted where comments, strings
    and character literals are masked, so what follows a test module is
    still read, and so is the rest of a module a test module is nested in.
    """
    blanked = masked(text)
    kept: list[str] = []
    at = 0
    for module in MODULE.finditer(blanked):
        # One inside a test module already cut goes with it.
        if module.group(3) != "{" or module.start() < at:
            continue
        if not only_under_test(cfgs(module, text)):
            continue
        kept.append(text[at : module.start()])
        at = block_end(blanked, module.end())
    kept.append(text[at:])
    return "".join(kept)


def own_files(src: Path) -> dict[Path, str]:
    """Give each file of a crate's own code under src, by path, without its tests."""
    tests = read_by_tests_alone(src)
    own: dict[Path, str] = {}
    for path in sorted(src.rglob("*.rs")):
        if path.resolve() in tests:
            continue
        try:
            own[path] = without_inline_tests(path.read_text())
        except RefusedError as error:
            raise error.within(path) from error
    return own


def own_code(src: Path) -> str:
    """Give a crate's own code under src, without what only its tests read."""
    return "\n".join(own_files(src).values())


if __name__ == "__main__":
    # Run alone, it names what only the tests of the crate under the src
    # it's given read, or with --own, each file of its own code and what's
    # left of it, so a planted crate can be held to what it declares.
    own = sys.argv[1] == "--own"
    given = Path(sys.argv[-1]).resolve()
    if own:
        for path, text in own_files(given).items():
            print(f"== {path.relative_to(given)}")
            print(text, end="")
    else:
        for name in sorted(str(file.relative_to(given)) for file in read_by_tests_alone(given)):
            print(name)
