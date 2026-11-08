"""Hold each list the launcher shows to its kind's one order (docs/design.md, Lists).

modules/quickshell/checks.nix runs this with the folders the launcher's places are in: the
launcher's own and the people module's. Each place named below sorts its rows by its kind's order
from launcher/Order.js, which tests/order.qml holds to the rule. No sort in those folders breaks a
tie A to Z by hand, since A to Z is only ever Order.js's last tie (RSCH-20261030-GB56HX found
Programs, Screens, Sound, Bluetooth and Connections falling back to it), and Qt's engine leaves out
the numeric reading a hand-made one asks for.
"""

from __future__ import annotations

import sys
from pathlib import Path

# Each place that lists things with nothing typed, by its file in the folders given, and the kind
# of list it is. A fixed set keeps the order it was designed in and sorts nothing, so none is here.
PLACES = {
    "Accessories.qml": "live",
    "Applications.qml": "returned",
    "Displays.qml": "choices",
    "Machine.qml": "amounts",
    "Networks.qml": "live",
    "Orchestrators.qml": "stream",
    "Person.js": "returned",
    "Quests.qml": "returned",
    "Recent.js": "stream",
    "Runs.qml": "stream",
    "Sound.qml": "choices",
    "Traffic.qml": "amounts",
    "Wireless.qml": "live",
}


def sorts(text: str) -> list[tuple[int, str]]:
    """Give each sort's line and what it sorts by, up to the parenthesis that closes it."""
    found = []
    start = text.find(".sort(")
    while start >= 0:
        depth = 0
        end = start + len(".sort")
        while end < len(text):
            depth += {"(": 1, ")": -1}.get(text[end], 0)
            end += 1
            if depth == 0:
                break
        found.append((text.count("\n", 0, start) + 1, text[start:end]))
        start = text.find(".sort(", end)
    return found


def failures(folders: list[Path]) -> list[str]:
    """Say each place that keeps another order than its kind's, and each A to Z made by hand."""
    files = {
        path.name: path
        for folder in folders
        for path in sorted(folder.iterdir())
        if path.suffix in {".qml", ".js"}
    }
    said = []
    for name, kind in PLACES.items():
        path = files.get(name)
        if path is None:
            said.append(f"{name} is named here, as a list of {kind}, but no folder given holds it")
        elif f"Order.{kind}(" not in path.read_text():
            said.append(f"{name} lists {kind}, but sorts by something other than Order.{kind}")
    for name, path in sorted(files.items()):
        if name == "Order.js":
            continue
        for line, by in sorts(path.read_text()):
            if "localeCompare" in by:
                said.append(
                    f"{name}:{line} sorts A to Z by hand; give its kind's order from Order.js, "
                    "whose last tie is A to Z"
                )
    return said


def main() -> int:
    """Check the folders given, and say every place out of its order."""
    said = failures([Path(folder) for folder in sys.argv[1:]])
    for one in said:
        print(one, file=sys.stderr)
    return 1 if said else 0


if __name__ == "__main__":
    sys.exit(main())
