"""Check Bluetooth's driver thaws its chip with a plain wake, never a reset.

    thaw-is-a-plain-wake SOURCE

SOURCE is drivers/bluetooth/btintel_pcie.c, as the machine's kernel builds it. A
hibernation's thaw that resets the chip leaves it in its bootloader, and the power-off
after refuses it, rolling the hibernation back with the laptop left on. So the thaw must
wake the chip as a thaw and reset nothing itself, and each reset in the wake it shares
must sit under a test a thaw can't meet.
"""

import re
import sys
from pathlib import Path

RESET = re.compile(r"btintel_pcie_(?:request_)?reset")
TOKEN = re.compile(r"[A-Za-z_]\w*|\S")

# What a test may compare and still never hold on a sound thaw: the wake's own event
# when it is a restore or a resume, a resume's target sleep, which is cleared before a
# hibernation begins, and the chip found in error, which a reset recovers. A thaw
# always follows a freeze, so anything else (the freeze, a record of the sleep, any
# event but these) may hold on one.
SAFE = re.compile(
    r"mesg\.event == PM_EVENT_(?:RESTORE|RESUME)"
    r"|pm_suspend_target_state == PM_SUSPEND_MEM"
    r"|btintel_pcie_in_(?:error|device_halt)\(data\)"
)

# A test a reset sits in the else of, which holds when that test fails.
ELSE = "the test before it fails"


def body(source: str, name: str) -> str:
    """Give the lines of one of the driver's functions, or nothing when it has none."""
    found = re.search(rf"^static int {name}\(.*?^\}}$", source, re.MULTILINE | re.DOTALL)
    return found.group() if found else ""


def unwrapped(test: str) -> str:
    """Take off the parentheses around all of a test."""
    test = test.strip()
    while test.startswith("(") and closing(test, 0) == len(test) - 1:
        test = test[1:-1].strip()
    return test


def closing(text: str, start: int) -> int:
    """Find where the parenthesis opening at start closes."""
    depth = 0
    for at, mark in enumerate(text[start:], start):
        depth += {"(": 1, ")": -1}.get(mark, 0)
        if depth == 0:
            return at
    sys.exit("Bluetooth's wake has a test this check can't read: " + text)


def parts(test: str, joiner: str) -> list[str]:
    """Split a test at each joiner outside its parentheses."""
    split, depth, start = [], 0, 0
    for at, mark in enumerate(test):
        depth += {"(": 1, ")": -1}.get(mark, 0)
        if depth == 0 and test.startswith(joiner, at):
            split.append(test[start:at])
            start = at + len(joiner)
    return [*split, test[start:]]


def holds_back(test: str) -> bool:
    """Tell whether a test can't hold on a sound thaw."""
    test = unwrapped(test)
    if len(either := parts(test, "||")) > 1:
        return all(holds_back(part) for part in either)
    if len(both := parts(test, "&&")) > 1:
        return any(holds_back(part) for part in both)
    return SAFE.fullmatch(test) is not None


class Reader:
    """Walks one C function, keeping the tests that hold where it has reached.

    Comments are read past, since they say nothing of what runs and may name any event.
    """

    def __init__(self, function: str) -> None:
        """Start at the function's first word."""
        self.text = re.sub(r"/\*.*?\*/|//[^\n]*", " ", function, flags=re.DOTALL)
        self.tokens = list(TOKEN.finditer(self.text))
        self.at = 0
        # The test each open brace opened under, empty for none, and the tests a
        # braceless if or else holds over the statement after it.
        self.held: list[str] = []
        self.pending: list[str] = []
        self.found: list[list[str]] = []

    def word(self, ahead: int = 0) -> str:
        """Give the word this many past where the reader is, or nothing past the end."""
        at = self.at + ahead
        return self.tokens[at].group() if at < len(self.tokens) else ""

    def test(self) -> str:
        """Read the parentheses opening at the next word, and give what they hold."""
        opening = self.tokens[self.at + 1].start()
        end = closing(self.text, opening)
        while self.at < len(self.tokens) and self.tokens[self.at].start() <= end:
            self.at += 1
        return self.text[opening + 1 : end].strip()

    def under(self, test: str) -> None:
        """Hold a test over the block after it, or the one statement when it has none."""
        if self.word() == "{":
            self.held.append(test)
            self.at += 1
        else:
            self.pending.append(test)

    def step(self) -> None:
        """Read one word, or one test with the word that opens it."""
        word, following = self.word(), self.word(1)
        if word == "else" and following != "if":
            self.at += 1
            self.under(ELSE)
        elif word == "if" and following == "(":
            self.under(self.test())
        elif word in {"for", "while", "switch"} and following == "(":
            self.test()
        elif RESET.fullmatch(word) and following == "(":
            self.found.append([test for test in self.held if test] + self.pending)
            self.test()
        else:
            if word == "{":
                self.held.append("")
            elif word == "}" and self.held:
                self.held.pop()
            elif word == ";":
                self.pending = []
            self.at += 1


def resets(function: str) -> list[list[str]]:
    """Give each reset of the chip in a function as the tests it sits under, outermost first."""
    reader = Reader(function)
    while reader.at < len(reader.tokens):
        reader.step()
    return reader.found


def main(argv: list[str]) -> None:
    """Fail, saying why, unless the thaw wakes the chip with no reset a thaw can meet."""
    source = Path(argv[0]).read_text()
    thaw = re.search(r"^\s*\.thaw = (\w+),$", source, re.MULTILINE)
    if not thaw:
        sys.exit("Bluetooth's driver no longer names its thaw where this check reads.")
    thawing = body(source, thaw.group(1))
    if not thawing:
        sys.exit(
            f"Bluetooth's driver names {thaw.group(1)} as its thaw, and this check can't find it."
        )
    refused = "and a hibernation's power-off after it can be refused."
    if "btintel_pcie_resume_event(dev, PMSG_THAW)" not in thawing:
        sys.exit(f"Bluetooth's thaw doesn't wake its chip as a thaw, so it can reset it, {refused}")
    if resets(thawing):
        sys.exit(f"Bluetooth's thaw resets its chip itself, {refused}")

    waking = resets(body(source, "btintel_pcie_resume_event"))
    if not waking:
        sys.exit("Bluetooth's wake no longer resets its chip where this check reads.")
    for tests in waking:
        if not tests:
            sys.exit(f"Bluetooth's wake resets its chip on every wake, so on a thaw, {refused}")
        if not any(test != ELSE and holds_back(test) for test in tests):
            when = " and ".join(" ".join(test.split()) for test in tests)
            sys.exit(f"Bluetooth's wake resets its chip when {when}, as a thaw can, {refused}")
    if not any(
        any(test != ELSE and holds_back(test) and "PM_EVENT_RESTORE" in test for test in tests)
        for tests in waking
    ):
        sys.exit(
            "Bluetooth's wake no longer resets its chip for a restore, so a hibernation "
            "comes back to a chip its firmware left."
        )


if __name__ == "__main__":
    main(sys.argv[1:])
