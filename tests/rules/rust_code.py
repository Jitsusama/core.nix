"""Tell a crate's own Rust code from what only its tests read, as rustc finds its modules.

A file is read only by tests when a module declared under cfgs that hold only
under test, such as `#[cfg(test)]` or `#[cfg(all(feature = "x", test))]`, is
read from it, or a module such a file declares is, or when it lies in the
folder of an inline test module, where what that module declares is read
from. A file's name says nothing either way: a module named `x_tests` may be
real code, and one named `golden` read by tests alone.
"""

import re
import sys
from pathlib import Path

# A module declared, with the attributes and doc lines above it, and whether
# its body follows inline.
MODULE = re.compile(r"(?m)^[ \t]*((?:(?:#\[.*\]|///.*)\n[ \t]*)*)(?:pub\S* )?mod (\w+)[ \t]*([;{])")

# An attribute line that is a cfg, and what it asks.
CFG = re.compile(r"^\s*#\[\s*cfg\s*\((.*)\)\s*\]\s*$")

# A cfg's words, strings and punctuation.
TOKEN = re.compile(r'"(?:\\.|[^"\\])*"|\w+|[(),=]')

# Where an attribute says a module is read from, in place of its name.
PATH = re.compile(r'#\[path = "([^"]+)"\]')


type Predicate = tuple[str, list["Predicate"] | None]


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


def only_under_test(attributes: str) -> bool:
    """Whether the cfgs among a module's attributes hold only where tests are built.

    Doc lines are left out, so one that mentions a cfg asks nothing, and
    several cfgs must all hold, so one that holds only under test is enough.
    """
    for line in attributes.splitlines():
        if cfg := CFG.match(line):
            tokens = TOKEN.findall(cfg.group(1))
            if tokens and holds_only_under_test(predicate(tokens)[0]):
                return True
    return False


def read_by_tests_alone(src: Path) -> set[Path]:
    """Give each Rust file under a crate's src that only its tests read, resolved.

    An inline module's folder is taken as though it lay at its file's top, so
    the modules a test module nested in another inline module declares are
    looked for a folder too high.
    """
    files = {path.resolve() for path in src.rglob("*.rs")}
    bins = (src / "bin").resolve()
    # A file read through a path attribute finds its modules beside it, as a
    # mod.rs does, whether or not only tests read it.
    beside = {
        (path.parent / given).resolve()
        for path in files
        for given in PATH.findall(path.read_text())
    }
    tests: set[Path] = set()
    folders: set[Path] = set()
    left = sorted(files)
    while left:
        path = left.pop()
        # A crate's roots and a mod.rs find their modules beside them; any
        # other file finds them in the folder named as it is.
        root = path.name in ("main.rs", "lib.rs", "mod.rs") or path.parent == bins
        here = path.parent if root or path in beside else path.with_suffix("")
        for attributes, name, body in MODULE.findall(path.read_text()):
            if not (only_under_test(attributes) or path in tests):
                continue
            if body == "{":
                folders.add(here / name)
                continue
            given = PATH.search(attributes)
            file = path.parent / given.group(1) if given else here / f"{name}.rs"
            if not given and not file.exists():
                file = here / name / "mod.rs"
            # A module whose cfg rules it out needn't have a file, and one
            # that's missing isn't read.
            if not file.exists():
                continue
            if (file := file.resolve()) not in tests:
                tests.add(file)
                left.append(file)
    return {
        file for file in files if file in tests or any(folder in file.parents for folder in folders)
    }


def without_inline_tests(text: str) -> str:
    """Give a file's text without the test modules written inline in it.

    Each runs to the first `}` at its own indent after it, as rustfmt sets
    a module's close, so what follows a test module is still read, and so
    is the rest of a module a test module is nested in.
    """
    kept: list[str] = []
    at = 0
    for module in MODULE.finditer(text):
        if module.group(3) != "{" or module.start() < at:
            continue
        if not only_under_test(module.group(1)):
            continue
        line_end = text.find("\n", module.end())
        line_end = len(text) if line_end < 0 else line_end
        if text[module.end() : line_end].strip() == "}":
            end = line_end
        else:
            whole = module.group(0)
            indent = whole[: len(whole) - len(whole.lstrip(" \t"))]
            close = re.compile(rf"(?m)^{re.escape(indent)}\}}").search(text, module.end())
            end = close.end() if close else len(text)
        kept.append(text[at : module.start()])
        at = end
    kept.append(text[at:])
    return "".join(kept)


def own_files(src: Path) -> dict[Path, str]:
    """Give each file of a crate's own code under src, by path, without its tests."""
    tests = read_by_tests_alone(src)
    return {
        path: without_inline_tests(path.read_text())
        for path in sorted(src.rglob("*.rs"))
        if path.resolve() not in tests
    }


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
