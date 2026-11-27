"""Agents.qml's asJoel names the very asks the agents daemon's AS_JOEL does.

The daemon does those asks only for the shell, and the shell sends one only from Joel's own
Return (Agents.qml's joelsAsk); an ask added to the daemon's alone would go from a walk as
it's served, refused only by the daemon, and one added to the shell's alone would be held
for nothing (round 000022 on #6796, F45 and F59). Run as `as-joel-agrees.py MAIN_RS AGENTS_QML`.
"""

import re
import sys
from pathlib import Path

QUOTED = re.compile(r'"([^"]*)"')
DAEMONS = re.compile(r"const AS_JOEL: \[&str; \d+\] = \[([^\]]*)\]")
SHELLS = re.compile(r"property var asJoel: \[([^\]]*)\]")


def listed(text: str, made: re.Pattern[str], where: str) -> list[str]:
    """The quoted names in the list `made` finds in `text`, or the run ends saying none."""
    found = made.search(text)
    if not found:
        sys.exit(f"{where} has no list matching {made.pattern!r}")
    return QUOTED.findall(found.group(1))


def main(daemon: Path, launcher: Path) -> None:
    """End the run, naming each side's extra asks, where the two lists differ."""
    daemons = listed(daemon.read_text(), DAEMONS, str(daemon))
    shells = listed(launcher.read_text(), SHELLS, str(launcher))
    if not daemons:
        sys.exit(f"{daemon}'s AS_JOEL names no ask")
    if len(set(shells)) != len(shells):
        sys.exit(f"{launcher}'s asJoel names one twice: {shells}")
    only_daemon = sorted(set(daemons) - set(shells))
    only_shell = sorted(set(shells) - set(daemons))
    if only_daemon or only_shell:
        sys.exit(f"only the daemon's AS_JOEL: {only_daemon}; only Agents' asJoel: {only_shell}")
    print(f"the shell and the daemon agree on {len(daemons)} asks done as Joel")


if __name__ == "__main__":
    main(Path(sys.argv[1]), Path(sys.argv[2]))
