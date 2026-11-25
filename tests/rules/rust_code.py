"""Tell a crate's own Rust code from what only its tests read, as rustc finds its modules.

A file is read only by tests when a module declared under a cfg that holds
only under test, `#[cfg(test)]` or `#[cfg(all(test, ...))]`, is read from it,
or a module such a file declares is, or when it lies in the folder of an
inline test module, where what that module declares is read from. A file's
name says nothing either way: a module named `x_tests` may be real code, and
one named `golden` read by tests alone.
"""

import re
import sys
from pathlib import Path

# A module declared, with the attributes and doc lines above it, and whether
# its body follows inline.
MODULE = re.compile(r"(?m)^[ \t]*((?:(?:#\[.*\]|///.*)\n[ \t]*)*)(?:pub\S* )?mod (\w+)[ \t]*([;{])")

# A cfg that holds only under test.
TEST = re.compile(r"#\[cfg\((?:all\()?test\b")

# Where an attribute says a module is read from, in place of its name.
PATH = re.compile(r'#\[path = "([^"]+)"\]')

# Where a file's inline test module starts, at its margin, past which a
# file holds only tests.
INLINE_TESTS = re.compile(
    r"(?m)^#\[cfg\((?:all\()?test\b.*\]\n(?:(?:#\[.*\]|///.*)\n)*(?:pub\S* )?mod \w+ \{"
)


def read_by_tests_alone(src: Path) -> set[Path]:
    """Give each Rust file under a crate's src that only its tests read, resolved."""
    files = {path.resolve() for path in src.rglob("*.rs")}
    bins = (src / "bin").resolve()
    tests: set[Path] = set()
    folders: set[Path] = set()
    left = sorted(files)
    while left:
        path = left.pop()
        # A crate's roots and a mod.rs find their modules beside them; any
        # other file finds them in the folder named as it is.
        root = path.name in ("main.rs", "lib.rs", "mod.rs") or path.parent == bins
        here = path.parent if root else path.with_suffix("")
        for attributes, name, body in MODULE.findall(path.read_text()):
            if not (TEST.search(attributes) or path in tests):
                continue
            if body == "{":
                folders.add(here / name)
                continue
            given = PATH.search(attributes)
            file = path.parent / given.group(1) if given else here / f"{name}.rs"
            if not given and not file.exists():
                file = here / name / "mod.rs"
            if (file := file.resolve()) not in tests:
                tests.add(file)
                left.append(file)
    return {
        file for file in files if file in tests or any(folder in file.parents for folder in folders)
    }


def own_code(src: Path) -> str:
    """Give a crate's own code under src, without what only its tests read."""
    tests = read_by_tests_alone(src)
    return "\n".join(
        INLINE_TESTS.split(path.read_text(), maxsplit=1)[0]
        for path in sorted(src.rglob("*.rs"))
        if path.resolve() not in tests
    )


if __name__ == "__main__":
    # Run alone, it names what only the tests of the crate under the src
    # it's given read, so a planted crate can be held to what it declares.
    given = Path(sys.argv[1]).resolve()
    for name in sorted(str(file.relative_to(given)) for file in read_by_tests_alone(given)):
        print(name)
