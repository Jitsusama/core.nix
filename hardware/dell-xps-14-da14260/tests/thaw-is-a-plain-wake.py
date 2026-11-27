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

# Comments, string and character literals, and preprocessor lines with any line a
# backslash carries one on to. None of them runs as the function's own code, and each may
# hold any word, a reset's name, a return or a brace, so the reader reads past them.
UNREAD = re.compile(
    r"/\*.*?\*/|//[^\n]*"
    r'|"(?:\\.|[^"\\\n])*"'
    r"|'(?:\\.|[^'\\\n])*'"
    r"|^[ \t]*#(?:/\*.*?\*/|\\\n|[^\n])*",
    re.DOTALL | re.MULTILINE,
)

# A call resets the chip where a word of its name, the words parted by underscores, is
# reset or flr: the driver's own request, its Bluetooth and IA cores, the ACPI method,
# PCI's, or one the check doesn't know by name, such as reset_chip() or flr(), which is
# still judged. One whose name asks after a reset or cancels one resets nothing, and
# skb_reset_*() sets a packet's headers back, never the chip.
RESETS = frozenset({"reset", "flr"})
ASKING = frozenset({"cancel", "cancelled", "done", "has", "in", "is", "pending", "status", "was"})
NOT_THE_CHIP = frozenset({"skb"})
# The reset work resets the chip only where it's queued, by any of the workqueue's calls
# that queue a work, now, later or on a given processor; cancelling it, flushing it or
# asking whether it's pending resets nothing.
QUEUEING = re.compile(r"(?:schedule|queue)(?:_delayed)?_work(?:_on)?|mod_delayed_work(?:_on)?")
RESET_WORK = re.compile(r"\breset_work\b")
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
# A test of the wake's own event, which says whether it holds on a restore.
EVENT = re.compile(r"mesg\.event (==|!=) PM_EVENT_(\w+)")
NOT = "not "
# What holds over a loop's or a switch's body in place of a test. Whatever the wake, a
# loop's body may not run at all, and a switch's runs only from the case its value
# matches, so a return only either reaches guards nothing past it: neither is ever met by
# every thaw, or held back on one.
LOOP = "(a loop)"
SWITCH = "(a switch)"
# What a switch's default case holds under, which the reader can't put as a test.
DEFAULT = "(default)"
# The words that leave a statement's block where it stands.
LEAVING = frozenset({"return", "break"})


def blank(found: re.Match[str]) -> str:
    """Give what stands in for a literal or a line read past, keeping its lines."""
    text = found.group()
    if text[0] in "\"'":
        return text[0] * 2
    return " " + "\n" * text.count("\n")


def unread(source: str) -> str:
    """Take out a source's comments, literals and preprocessor lines, keeping its lines."""
    return UNREAD.sub(blank, source)


def body(source: str, name: str) -> str:
    """Give the lines of one of the driver's functions, or nothing when it has none."""
    found = re.search(rf"^static int {name}\(.*?^\}}[ \t]*$", source, re.MULTILINE | re.DOTALL)
    return found.group() if found else ""


def resetting(name: str) -> bool:
    """Tell whether a call by this name resets the chip, by the words of its name."""
    words = name.strip("_").split("_")
    resets_by_name = not RESETS.isdisjoint(words)
    return resets_by_name and ASKING.isdisjoint(words) and words[0] not in NOT_THE_CHIP


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


def on_restore(test: str) -> bool | None:
    """Tell whether a test holds when the wake is a restore, or None where it can't say."""
    test = unwrapped(test)
    if test.startswith(NOT):
        held = on_restore(test[len(NOT) :])
        return None if held is None else not held
    if len(either := parts(test, "||")) > 1:
        return joined([on_restore(part) for part in either], settles=True)
    if len(both := parts(test, "&&")) > 1:
        return joined([on_restore(part) for part in both], settles=False)
    event = EVENT.fullmatch(test)
    if event is None:
        return None
    return (event.group(2) == "RESTORE") == (event.group(1) == "==")


def joined(said: list[bool | None], *, settles: bool) -> bool | None:
    """Give what parts joined say, where one part saying settles decides it alone.

    An or's parts are settled by one that holds, an and's by one that fails; otherwise
    the parts decide it only where every one says.
    """
    if settles in said:
        return settles
    if None in said:
        return None
    return not settles


def restores(tests: list[str]) -> bool:
    """Tell whether a reset under these tests runs on a restore.

    A test of its own must say it holds on one, since a guard left by an early return only
    says what got past it, and none of the tests may say it fails on one.
    """
    said = [on_restore(test) for test in tests]
    return False not in said and any(
        held is True and not test.startswith(NOT) for test, held in zip(tests, said, strict=True)
    )


@dataclass(frozen=True)
class Held:
    """A test that holds over a statement or a block, the word that opened it, and its value.

    The opener is if, else, loop or switch; only a switch has a value, what it switches on.
    """

    test: str
    opener: str
    value: str = ""


def tests(held: list[Held]) -> list[str]:
    """Give the tests of what holds."""
    return [each.test for each in held]


def last_if(held: list[Held]) -> tuple[list[Held], str] | None:
    """Give the innermost if among what holds, as what held around it and its own test.

    An else is that if's, so it holds where the if's surroundings did and its test failed.
    """
    for at in range(len(held) - 1, -1, -1):
        if held[at].opener == "if":
            return held[:at], held[at].test
    return None


@dataclass
class Block:
    """One pair of braces: what it runs under, and the early returns before a word.

    A switch's block also keeps what it switches on and the cases that reach where the
    reader is, which fall through until a break or a return ends them.
    """

    held: list[Held]
    guards: list[str] = field(default_factory=list)
    returns: bool = False
    switch: str | None = None
    cases: list[str] = field(default_factory=list)


class Reader:
    """Walks one C function, keeping the tests that hold where it has reached.

    Comments, literals and preprocessor lines are read past, since they say nothing of what
    runs and may name any event or call.
    """

    def __init__(self, function: str) -> None:
        """Start at the function's first word."""
        self.text = unread(function)
        self.tokens = list(TOKEN.finditer(self.text))
        self.at = 0
        # Each open brace's block; what a braceless if, else, loop or switch holds over
        # the statement after it; the if an else would be the else of; which word leaves
        # the statement being read, if one does.
        self.blocks: list[Block] = []
        self.pending: list[Held] = []
        self.closed: tuple[list[Held], str] | None = None
        self.leaving = ""
        self.found: list[list[str]] = []

    def word(self, ahead: int = 0) -> str:
        """Give the word this many past where the reader is, or nothing past the end."""
        at = self.at + ahead
        return self.tokens[at].group() if at < len(self.tokens) else ""

    def holding(self) -> list[str]:
        """Give every test that holds where the reader is, outermost first."""
        holds: list[str] = []
        for block in self.blocks:
            holds += tests(block.held)
            if block.cases:
                holds.append(" || ".join(block.cases))
            holds += block.guards
        return holds + tests(self.pending)

    def resets_at(self, at: int) -> bool:
        """Tell whether the word at a place calls what resets the chip."""
        word = self.tokens[at].group()
        if at + 1 >= len(self.tokens) or self.tokens[at + 1].group() != "(":
            return False
        if QUEUEING.fullmatch(word):
            opening = self.tokens[at + 1].start()
            return RESET_WORK.search(self.text, opening, closing(self.text, opening)) is not None
        return resetting(word)

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

    def opener(self, word: str) -> None:
        """Read an if's, a loop's or a switch's parentheses, and hold what they open over."""
        test = self.test()
        if word == "if":
            self.pending.append(Held(test, "if"))
        elif word == "switch":
            self.pending.append(Held(SWITCH, "switch", test))
        else:
            self.pending.append(Held(LOOP, "loop"))

    def otherwise(self) -> None:
        """Read an else, which holds where its if's surroundings did and the if's test failed."""
        self.at += 1
        if self.closed is None:
            self.pending.append(Held(NOT + "(the test before)", "else"))
        else:
            around, test = self.closed
            self.pending = [*around, Held(negated([test]), "else")]
        self.closed = None

    def labelled(self) -> None:
        """Read a case's label, or a default's, as one more case that reaches on."""
        block = self.blocks[-1] if self.blocks else None
        if block is None or block.switch is None or self.pending:
            sys.exit("Bluetooth's wake has a case this check can't read, outside its switch.")
        start = self.at
        while self.at < len(self.tokens) and self.word() != ":":
            self.at += 1
        if self.at == len(self.tokens):
            sys.exit("Bluetooth's wake has a case this check can't read, with no colon after it.")
        label = self.text[self.tokens[start].end() : self.tokens[self.at].start()].strip()
        self.at += 1
        is_default = self.tokens[start].group() == "default"
        block.cases.append(DEFAULT if is_default else f"{block.switch} == {label}")
        # A case jumped to never passed the early returns before it.
        block.guards = []

    def opened(self) -> None:
        """Open a block under whatever a braceless if, else, loop or switch held over it."""
        self.at += 1
        if self.pending and self.pending[-1].opener == "switch":
            self.blocks.append(Block(self.pending[:-1], switch=self.pending[-1].value))
        else:
            self.blocks.append(Block(self.pending))
        self.pending = []

    def left(self, block: Block, word: str) -> None:
        """Leave a block where the reader stands in it, by a return or a break.

        Past a return, the rest of a block that isn't a switch's runs nowhere. A switch's
        return or break ends only the cases that reached it, never the switch: what follows
        runs from the next case.
        """
        if block.switch is None:
            block.returns = block.returns or word == "return"
        else:
            block.cases = []
            block.guards = []

    def shut(self) -> None:
        """Close a block; past one that always returns, its tests have failed."""
        self.at += 1
        if not self.blocks:
            return
        block = self.blocks.pop()
        self.closed = last_if(block.held)
        if not (block.returns and self.blocks):
            return
        if block.held:
            self.blocks[-1].guards.append(negated(tests(block.held)))
        else:
            # A block under no test returns from where it stands in the block around it.
            self.left(self.blocks[-1], "return")

    def ended(self) -> None:
        """End a statement; past a return under a braceless if, its tests have failed."""
        self.at += 1
        if self.leaving and self.blocks:
            if not self.pending:
                self.left(self.blocks[-1], self.leaving)
            elif self.leaving == "return":
                self.blocks[-1].guards.append(negated(tests(self.pending)))
        self.closed = last_if(self.pending)
        self.pending = []
        self.leaving = ""

    def step(self) -> None:
        """Read one word, or one test with the word that opens it."""
        word, following = self.word(), self.word(1)
        if word == "else":
            self.otherwise()
        elif word in {"if", "for", "while", "switch"} and following == "(":
            self.opener(word)
        elif word in {"case", "default"}:
            self.labelled()
        elif self.resets_at(self.at):
            self.found.append(self.holding())
            self.test()
        elif word == "{":
            self.opened()
        elif word == "}":
            self.shut()
        elif word == ";":
            self.ended()
        else:
            if word in LEAVING and not self.leaving:
                self.leaving = word
            self.at += 1


def resets(function: str) -> list[list[str]]:
    """Give each reset of the chip in a function as the tests it sits under, outermost first."""
    reader = Reader(function)
    while reader.at < len(reader.tokens):
        reader.step()
    return reader.found


def main(argv: list[str]) -> None:
    """Fail, saying why, unless the thaw wakes the chip with no reset a thaw can meet."""
    source = unread(Path(argv[0]).read_text())
    thaw = re.search(r"^\s*\.thaw = (\w+),[ \t]*$", source, re.MULTILINE)
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
    for held in waking:
        if not held:
            sys.exit(f"Bluetooth's wake resets its chip on every wake, so on a thaw, {refused}")
        if not any(holds_back(test) for test in held):
            when = " and ".join(" ".join(test.split()) for test in held)
            sys.exit(f"Bluetooth's wake resets its chip when {when}, as a thaw can, {refused}")
    if not any(restores(held) for held in waking):
        sys.exit(
            "Bluetooth's wake no longer resets its chip for a restore, so a hibernation "
            "comes back to a chip its firmware left."
        )


if __name__ == "__main__":
    main(sys.argv[1:])
