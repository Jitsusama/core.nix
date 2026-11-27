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
from dataclasses import dataclass, field
from pathlib import Path

# Any call whose name says it resets or takes a function-level reset (the driver's own
# request, its Bluetooth and IA cores, the ACPI method, PCI's), and the reset work the
# driver queues, count as a reset: one the check doesn't know by name is still judged.
RESET = re.compile(r"[a-z_][a-z0-9_]*(?:reset|flr)[a-z0-9_]*")
RESET_WORK = "reset_work"
TOKEN = re.compile(r"[A-Za-z_]\w*|\S")

# What a test may compare and still never hold on a sound thaw: the wake's own event
# when it is a restore or a resume, or anything but a thaw, a resume's target sleep,
# which is cleared before a hibernation begins, and the chip found in error, which a
# reset recovers. A thaw always follows a freeze, so anything else (the freeze, a record
# of the sleep, any other event) may hold on one.
SAFE = re.compile(
    r"mesg\.event == PM_EVENT_(?:RESTORE|RESUME)"
    r"|mesg\.event != PM_EVENT_THAW"
    r"|pm_suspend_target_state == PM_SUSPEND_MEM"
    r"|btintel_pcie_in_(?:error|device_halt)\(data\)"
)
# What every thaw meets, so that a reset past an early return under it never runs on one.
THAW = re.compile(r"mesg\.event == PM_EVENT_THAW")
NOT = "not "


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


def negated(tests: list[str]) -> str:
    """Give the test that holds when the tests given don't all hold."""
    return NOT + "(" + " && ".join(f"({test})" for test in tests) + ")"


def holds_back(test: str) -> bool:
    """Tell whether a test can't hold on a sound thaw."""
    test = unwrapped(test)
    if test.startswith(NOT):
        return always_on_thaw(test[len(NOT) :])
    if len(either := parts(test, "||")) > 1:
        return all(holds_back(part) for part in either)
    if len(both := parts(test, "&&")) > 1:
        return any(holds_back(part) for part in both)
    return SAFE.fullmatch(test) is not None


def always_on_thaw(test: str) -> bool:
    """Tell whether every thaw meets a test, so its failing never holds on one."""
    test = unwrapped(test)
    if len(either := parts(test, "||")) > 1:
        return any(always_on_thaw(part) for part in either)
    if len(both := parts(test, "&&")) > 1:
        return all(always_on_thaw(part) for part in both)
    return THAW.fullmatch(test) is not None


@dataclass
class Block:
    """One pair of braces: the tests it runs under, and the early returns before a word."""

    tests: list[str]
    guards: list[str] = field(default_factory=list)
    returns: bool = False


class Reader:
    """Walks one C function, keeping the tests that hold where it has reached.

    Comments are read past, since they say nothing of what runs and may name any event.
    """

    def __init__(self, function: str) -> None:
        """Start at the function's first word."""
        self.text = re.sub(r"/\*.*?\*/|//[^\n]*", " ", function, flags=re.DOTALL)
        self.tokens = list(TOKEN.finditer(self.text))
        self.at = 0
        # Each open brace's block; the tests a braceless if, else or loop holds over the
        # statement after it; the test an else would be the else of; whether the
        # statement being read returns.
        self.blocks: list[Block] = []
        self.pending: list[str] = []
        self.closed: list[str] = []
        self.returning = False
        self.found: list[list[str]] = []

    def word(self, ahead: int = 0) -> str:
        """Give the word this many past where the reader is, or nothing past the end."""
        at = self.at + ahead
        return self.tokens[at].group() if at < len(self.tokens) else ""

    def holding(self) -> list[str]:
        """Give every test that holds where the reader is, outermost first."""
        return [test for block in self.blocks for test in block.tests + block.guards] + [
            *self.pending
        ]

    def resets_at(self, at: int) -> bool:
        """Tell whether the word at a place resets the chip."""
        word = self.tokens[at].group()
        calls = at + 1 < len(self.tokens) and self.tokens[at + 1].group() == "("
        return word == RESET_WORK or (calls and RESET.fullmatch(word) is not None)

    def test(self) -> str:
        """Read the parentheses opening at the next word, and give what they hold.

        A reset called inside them runs under what holds outside them, so it is found
        with those tests.
        """
        opening = self.tokens[self.at + 1].start()
        end = closing(self.text, opening)
        self.at += 1
        while self.at < len(self.tokens) and self.tokens[self.at].start() <= end:
            if self.resets_at(self.at):
                self.found.append(self.holding())
            self.at += 1
        return self.text[opening + 1 : end].strip()

    def otherwise(self) -> None:
        """Read an else, which holds when the test it is the else of fails."""
        self.at += 1
        self.pending.append(negated(self.closed) if self.closed else NOT + "(the test before)")

    def opened(self) -> None:
        """Open a block under whatever a braceless if, else or loop held over it."""
        self.blocks.append(Block(self.pending))
        self.pending = []
        self.at += 1

    def shut(self) -> None:
        """Close a block; past one that always returns, its tests have failed."""
        self.at += 1
        if not self.blocks:
            return
        block = self.blocks.pop()
        self.closed = block.tests
        if block.returns and block.tests and self.blocks:
            self.blocks[-1].guards.append(negated(block.tests))

    def ended(self) -> None:
        """End a statement; past a return under a braceless if, its tests have failed."""
        self.at += 1
        if self.returning and self.blocks:
            if self.pending:
                self.blocks[-1].guards.append(negated(self.pending))
            else:
                self.blocks[-1].returns = True
        if self.pending:
            self.closed = self.pending
        self.pending = []
        self.returning = False

    def step(self) -> None:
        """Read one word, or one test with the word that opens it."""
        word, following = self.word(), self.word(1)
        if word == "else":
            self.otherwise()
        elif word == "if" and following == "(":
            self.pending.append(self.test())
        elif word in {"for", "while", "switch"} and following == "(":
            self.test()
        elif self.resets_at(self.at):
            self.found.append(self.holding())
            if following == "(":
                self.test()
            else:
                self.at += 1
        elif word == "{":
            self.opened()
        elif word == "}":
            self.shut()
        elif word == ";":
            self.ended()
        else:
            self.returning = self.returning or word == "return"
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
        if not any(holds_back(test) for test in tests):
            when = " and ".join(" ".join(test.split()) for test in tests)
            sys.exit(f"Bluetooth's wake resets its chip when {when}, as a thaw can, {refused}")
    if not any(
        any(holds_back(test) and "PM_EVENT_RESTORE" in test for test in tests) for tests in waking
    ):
        sys.exit(
            "Bluetooth's wake no longer resets its chip for a restore, so a hibernation "
            "comes back to a chip its firmware left."
        )


if __name__ == "__main__":
    main(sys.argv[1:])
