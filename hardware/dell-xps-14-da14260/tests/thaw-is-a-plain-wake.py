"""Check Bluetooth's driver thaws its chip with a plain wake, never a reset.

    thaw-is-a-plain-wake SOURCE

SOURCE is drivers/bluetooth/btintel_pcie.c, as the machine's kernel builds it. A
hibernation's thaw that resets the chip leaves it in its bootloader, and the power-off
after refuses it, rolling the hibernation back with the laptop left on. So the thaw must
wake the chip as a thaw and reset nothing itself, and each reset in the wake it shares
must sit under a test a thaw can't meet. What the reader can't model, a goto, a label or
a preprocessor conditional, it refuses rather than guess at.
"""

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

# Comments, string and character literals, and preprocessor lines with any line a
# backslash carries one on to. None of them runs as the function's own code, and each may
# hold any word, a reset's name, a return or a brace, so the reader reads past them. A
# conditional's line is the one kind kept, as a mark the reader refuses: it decides which
# of the lines around it a kernel builds, and the reader can't tell which.
UNREAD = re.compile(
    r"/\*.*?\*/|//[^\n]*"
    r'|"(?:\\.|[^"\\\n])*"'
    r"|'(?:\\.|[^'\\\n])*'"
    r"|^[ \t]*#(?:/\*.*?\*/|\\\n|[^\n])*",
    re.DOTALL | re.MULTILINE,
)
CONDITIONAL = re.compile(r"[ \t]*#[ \t]*(if|ifdef|ifndef|elif|else)\b")
MARK = "#"

# A call resets the chip where a word of its name, the words parted by underscores and
# read in lower case, so a macro's as well, holds reset or is flr: the driver's own
# request, its Bluetooth and IA cores, the ACPI method, PCI's, or one the check doesn't
# know by name, such as reset_chip(), hwreset() or flr(), which is still judged. One whose
# name asks after a reset or cancels one resets nothing, but only where it reads as a
# question: a word of asking just before its first reset word, as in_reset or cancel_reset
# have, or as its last, as reset_pending has. Anywhere else such a word may say how a
# reset is done, as reset_in_band's does. And skb_reset_*() sets a packet's headers back,
# never the chip.
RESET = "reset"
FLR = "flr"
ASKING = frozenset({"cancel", "cancelled", "done", "has", "in", "is", "pending", "status", "was"})
NOT_THE_CHIP = frozenset({"skb"})
# The reset work resets the chip under any call it is handed to, the workqueue's, a kernel
# thread's or the driver's own helper, but those that cancel it, flush it, stop it or ask
# after it. So a call the check doesn't know is counted, rather than let a new way of
# queueing it pass.
CALM = re.compile(
    r"(?:cancel|flush|disable)_\w*work\w*|work_pending|delayed_work_pending|work_busy"
)
RESET_WORK = re.compile(r"\breset_work\b")
TOKEN = re.compile(r"[A-Za-z_]\w*|\S")
NAME = re.compile(r"[A-Za-z_]\w*")

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
# What an early return leaves held over the rest of its block, which says only what got
# past it, never what the statements past it test themselves.
GUARD = "guard"
# The words that open a test's parentheses and hold it over what follows.
OPENERS = frozenset({"if", "for", "while", "switch"})


def blank(found: re.Match[str]) -> str:
    """Give what stands in for a literal or a line read past, keeping its lines."""
    text = found.group()
    if text[0] in "\"'":
        return text[0] * 2
    lines = "\n" * text.count("\n")
    if conditional := CONDITIONAL.match(text):
        return MARK + conditional.group(1) + lines
    return " " + lines


def unread(source: str) -> str:
    """Take out a source's comments, literals and preprocessor lines, keeping its lines.

    A conditional's line is left as its mark and its word, such as #ifdef.
    """
    return UNREAD.sub(blank, source)


def body(source: str, name: str) -> str:
    """Give the lines of one of the driver's functions, or nothing when it has none."""
    found = re.search(rf"^static int {name}\(.*?^\}}[ \t]*$", source, re.MULTILINE | re.DOTALL)
    return found.group() if found else ""


def resetting(name: str) -> bool:
    """Tell whether a call by this name resets the chip, by the words of its name."""
    words = name.lower().strip("_").split("_")
    first = next((at for at, word in enumerate(words) if RESET in word or word == FLR), None)
    if first is None or words[0] in NOT_THE_CHIP:
        return False
    asks = (first > 0 and words[first - 1] in ASKING) or words[-1] in ASKING
    return not asks


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


@dataclass(frozen=True)
class Held:
    """A test that holds over a statement or a block, the word that opened it, and its value.

    The opener is if, else, loop, switch or case, or guard for what an early return left;
    only a switch has a value, what it switches on.
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


def restores(held: list[Held]) -> bool:
    """Tell whether a reset under what holds runs on a restore.

    None of its tests may say it fails on one, and a test of its own must say it holds on
    one, a negation as well, such as an else past a thaw's test. A guard left by an early
    return only says what got past it, so it speaks for the reset only where the reset has
    no test of its own.
    """
    said = [(each.opener, on_restore(each.test)) for each in held]
    if any(says is False for _, says in said):
        return False
    own = [says for opener, says in said if opener != GUARD]
    return True in (own or [says for _, says in said])


@dataclass
class Block:
    """One pair of braces: what it runs under, the early returns before a word, and what left.

    A block is left by the return or the break that ends it where it stands. A switch's
    block also keeps what it switches on, the cases that reach where the reader is, which
    fall through until a break or a return ends them, those of them no break has passed
    since their label, and those that returned with none passed.
    """

    held: list[Held]
    guards: list[str] = field(default_factory=list)
    left_by: str = ""
    switch: str | None = None
    cases: list[str] = field(default_factory=list)
    unbroken: list[str] = field(default_factory=list)
    returned: list[str] = field(default_factory=list)


class Reader:
    """Walks one C function, keeping the tests that hold where it has reached.

    The function comes with its comments, literals and preprocessor lines taken out, since
    they say nothing of what runs and may name any event or call. What the reader can't
    follow, it refuses: a goto or a label, whose jump skips what the reader reads as
    running, and a preprocessor conditional, which decides which of the lines a kernel has.
    """

    def __init__(self, function: str) -> None:
        """Start at the function's first brace, past its name and parameters.

        Its name and parameters, followed by its brace, would read as a loop's macro.
        """
        self.text = function
        self.tokens = list(TOKEN.finditer(self.text))
        self.at = next((at for at, token in enumerate(self.tokens) if token.group() == "{"), 0)
        # Each open brace's block; what a braceless if, else, loop or switch holds over
        # the statement after it; the if an else would be the else of; which word leaves
        # the statement being read, if one does; how many of its question marks wait for
        # their colons, so that a colon past them is read as no label's.
        self.blocks: list[Block] = []
        self.pending: list[Held] = []
        self.closed: tuple[list[Held], str] | None = None
        self.leaving = ""
        self.asking = 0
        self.found: list[list[Held]] = []

    def word(self, ahead: int = 0) -> str:
        """Give the word this many past where the reader is, or nothing past the end."""
        at = self.at + ahead
        return self.tokens[at].group() if at < len(self.tokens) else ""

    def holding(self) -> list[Held]:
        """Give everything that holds where the reader is, outermost first."""
        holds: list[Held] = []
        for block in self.blocks:
            holds += block.held
            if block.cases:
                holds.append(Held(" || ".join(block.cases), "case"))
            holds += [Held(guard, GUARD) for guard in block.guards]
        return holds + self.pending

    def resets_at(self, at: int) -> bool:
        """Tell whether the word at a place calls what resets the chip."""
        word = self.tokens[at].group()
        calls = at + 1 < len(self.tokens) and self.tokens[at + 1].group() == "("
        if not (calls and NAME.fullmatch(word)):
            return False
        opening = self.tokens[at + 1].start()
        handed = RESET_WORK.search(self.text, opening, closing(self.text, opening))
        return (handed is not None and not CALM.fullmatch(word)) or resetting(word)

    def refused(self, word: str, following: str) -> str:
        """Give why the reader can't read on from a word, or nothing where it can."""
        cannot = "Bluetooth's wake now has"
        if word == "goto":
            return f"{cannot} a goto this check can't read, so it can't tell what the jump passes."
        if word == MARK:
            return (
                f"{cannot} a preprocessor conditional, #{following}, this check can't read, so "
                "it can't tell which of its lines a kernel builds."
            )
        if following == ":" and not self.asking and NAME.fullmatch(word):
            return (
                f"{cannot} a label, {word}, this check can't read, so it can't tell what jumps "
                "to it."
            )
        return ""

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

    def called(self) -> None:
        """Read a call, and hold a loop over what follows it where the call is a loop's macro.

        A call followed straight by a brace or a word, as list_for_each_entry(...) { or
        for_each_set_bit(...) is, opens a body that may run any number of times, or none.
        """
        if self.resets_at(self.at):
            self.found.append(self.holding())
        self.test()
        if self.word() == "{" or NAME.fullmatch(self.word()):
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
        case = DEFAULT if is_default else f"{block.switch} == {label}"
        block.cases.append(case)
        block.unbroken.append(case)
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
        runs from the next case. But a case that returned with no break passed since its
        label keeps its value from what follows the switch, as a thaw's case may.
        """
        if block.switch is None:
            block.left_by = block.left_by or word
            return
        if word == "return":
            block.returned += [case for case in block.unbroken if case != DEFAULT]
        block.cases = []
        block.guards = []
        block.unbroken = []

    def broke(self) -> None:
        """Read a break, which takes a switch's cases past it, unless a loop holds it."""
        if any(each.opener in {"loop", "switch"} for each in self.pending):
            return
        for block in reversed(self.blocks):
            if block.switch is not None:
                block.unbroken = []
                return
            if any(each.opener == "loop" for each in block.held):
                return

    def shut(self) -> None:
        """Close a block; past one that always returns, its tests have failed.

        Past a switch, so has each case that returned with no break passed.
        """
        self.at += 1
        if not self.blocks:
            return
        block = self.blocks.pop()
        self.closed = last_if(block.held)
        if not self.blocks:
            return
        around = self.blocks[-1]
        around.guards += [negated([*tests(block.held), case]) for case in block.returned]
        if not block.left_by:
            return
        if not block.held:
            # A block under no test leaves the block around it where it stands in it.
            self.left(around, block.left_by)
        elif block.left_by == "return":
            around.guards.append(negated(tests(block.held)))

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
        self.asking = 0

    def went(self, word: str) -> None:
        """Read a word that opens nothing: a return, a break, a question's marks or another."""
        self.at += 1
        if word in LEAVING and not self.leaving:
            self.leaving = word
        if word == "break":
            self.broke()
        elif word == "?":
            self.asking += 1
        elif word == ":" and self.asking:
            self.asking -= 1

    def step(self) -> None:
        """Read one word, or one test or call with the word that opens it."""
        word, following = self.word(), self.word(1)
        marks = {"else": self.otherwise, "{": self.opened, "}": self.shut, ";": self.ended}
        if word in marks:
            marks[word]()
        elif word in {"case", "default"}:
            self.labelled()
        elif why := self.refused(word, following):
            sys.exit(why)
        elif word in OPENERS and following == "(":
            self.opener(word)
        elif following == "(" and word not in LEAVING and NAME.fullmatch(word):
            self.called()
        else:
            self.went(word)


def resets(function: str) -> list[list[Held]]:
    """Give each reset of the chip in a function as what it sits under, outermost first.

    The function comes with its comments, literals and preprocessor lines taken out.
    """
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
        if not any(holds_back(test) for test in tests(held)):
            when = " and ".join(" ".join(test.split()) for test in tests(held))
            sys.exit(f"Bluetooth's wake resets its chip when {when}, as a thaw can, {refused}")
    if not any(restores(held) for held in waking):
        sys.exit(
            "Bluetooth's wake no longer resets its chip for a restore, so a hibernation "
            "comes back to a chip its firmware left."
        )


if __name__ == "__main__":
    main(sys.argv[1:])
