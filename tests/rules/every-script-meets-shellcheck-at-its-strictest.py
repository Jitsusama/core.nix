"""Check that every shell script meets shellcheck's optional checks that find faults.

    every-script-meets-shellcheck-at-its-strictest TREE WAITING [FILE...]

treefmt runs shellcheck at its defaults and the optional checks no script
breaks. Four more find real faults but are broken in many scripts today: a
command's failure hidden inside another (check-extra-masked-returns), a
function whose `set -e` its caller turns off (check-set-e-suppressed), an
uppercase variable nothing sets (check-unassigned-uppercase), and a `case`
with no default (add-default-case). This runs shellcheck with those four over
TREE's `*.sh`, `*.bash` and `.envrc` files, and each FILE named after WAITING,
and counts what they find in each.

WAITING is JSON naming, for each file that broke them before the rule, how many
findings it holds while each is fixed. A file holding more fails, naming each;
a file holding fewer fails until its count is lowered, so the list only
shrinks. A finding that's right as it is takes a `# shellcheck disable=` with
its reason. Every failure is printed before the check exits 1.
"""

import json
import shutil
import subprocess
import sys
from pathlib import Path

CHECKS = (
    "add-default-case,check-extra-masked-returns,check-set-e-suppressed,check-unassigned-uppercase"
)
# What those four checks say, so shellcheck's defaults, which treefmt holds,
# aren't counted twice.
CODES = "SC2154,SC2249,SC2310,SC2311,SC2312"


def scripts(tree: Path, named: list[str]) -> list[str]:
    """Give each script shellcheck reads in TREE, as a path from TREE."""
    found = {
        path.relative_to(tree).as_posix()
        for path in tree.rglob("*")
        if (path.suffix in {".sh", ".bash"} or path.name == ".envrc") and path.is_file()
    }
    return sorted(found | set(named))


def findings(tree: Path, files: list[str]) -> dict[str, list[str]]:
    """Give each file's findings from the four checks, each as where and what."""
    if not files:
        return {}
    shellcheck = shutil.which("shellcheck")
    if shellcheck is None:
        sys.exit("shellcheck isn't on the PATH, which the check's derivation gives it.")
    ran = subprocess.run(
        [shellcheck, f"--enable={CHECKS}", f"--include={CODES}", "--format=json1", *files],
        cwd=tree,
        capture_output=True,
        text=True,
        check=False,
    )
    if not ran.stdout:
        sys.exit(f"shellcheck said nothing it could be read by: {ran.stderr}")
    found: dict[str, list[str]] = {}
    for comment in json.loads(ran.stdout)["comments"]:
        found.setdefault(comment["file"], []).append(
            f"{comment['file']}:{comment['line']}: SC{comment['code']} {comment['message']}"
        )
    return found


def faults(count: int) -> str:
    """Say how many findings there are, in words that agree with the count."""
    return f"{count} finding{'' if count == 1 else 's'}"


def main() -> None:
    """Say each file holding more findings than it waits with, and each holding fewer."""
    if len(sys.argv) < 3:  # noqa: PLR2004, the program's name, the tree and the list
        sys.exit("usage: every-script-meets-shellcheck-at-its-strictest TREE WAITING [FILE...]")
    tree, waiting_file = Path(sys.argv[1]), Path(sys.argv[2])
    waiting: dict[str, int] = json.loads(waiting_file.read_text())
    files = scripts(tree, sys.argv[3:])
    found = findings(tree, files)
    said: list[str] = []
    for path, held in sorted(found.items()):
        if len(held) > waiting.get(path, 0):
            said.append(
                f"{path} holds {faults(len(held))}, {waiting.get(path, 0)} of them waiting: fix "
                "each, or disable it on its line with the reason it's right as it is."
            )
            said.extend(f"  {line}" for line in held)
    said.extend(
        f"{path} waits with {faults(count)} but holds {len(found.get(path, []))}: lower its "
        "count on the waiting list, or take it off."
        for path, count in sorted(waiting.items())
        if len(found.get(path, [])) < count
    )
    for failure in said:
        print(failure, file=sys.stderr)
    total = sum(len(held) for held in found.values())
    print(f"{faults(total)} in {len(found)} of {len(files)} scripts.")
    sys.exit(1 if said else 0)


if __name__ == "__main__":
    main()
