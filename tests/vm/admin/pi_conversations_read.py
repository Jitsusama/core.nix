"""What pi-conversations-come-back's failures read from the machine, said on one line.

Only that VM test's driver is given this, so a change here moves no other VM test; its cases are
pi-conversations-read-test.py, which run apart from any machine.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

# What coreutils' timeout exits with when its command ran out of time, named as heard.py in
# tests/desktop names it.
CUT_SHORT = 124
# What a host may write that would read as the quoting's own: a backslash as the start of an
# escape, a pipe as the line break failure_read writes between lines.
QUOTED = {"\\": "\\\\", "|": "\\|"}


def escaped(line: str) -> str:
    """Give a line with each character that isn't printable written as its escape.

    That takes in every control code a terminal acts on, and invisible ones too, such as a
    no-break space. A backslash and a pipe are escaped as well, so the line reads back one way.
    """
    return "".join(
        QUOTED.get(c) or (c if c.isprintable() else c.encode("unicode_escape").decode())
        for c in line
    )


def failure_read(status: int, out: str, seconds: int) -> str:
    """Give what a command a failure reads printed, on one line, or why it can't be read.

    The driver's execute ends a command at its timeout with timeout's own exit, so that is said
    as running out of time. Lines are cut only at a newline, and every other character a
    terminal would act on, a carriage return or a journal's colours among them, is written as
    its escape, so nothing the host wrote can garble the failure that quotes it. Only a line
    that is empty or all spaces is left out; one holding a control code alone is kept.
    """
    said = " | ".join(escaped(line) for line in out.split("\n") if line.strip(" "))
    if status == CUT_SHORT:
        return f"nothing whole, as it ran out of its {seconds} seconds: {said or 'saying nothing'}"
    if status != 0:
        return f"nothing, as it exited {status}: {said or 'saying nothing'}"
    return said or "nothing at all"


def read_for_failure(run: Callable[[], tuple[int, str]], seconds: int) -> str:
    """Give what a read run for a failure printed, as failure_read says it, raising nothing.

    A read that raises says so, since a failure's read mustn't hide the failure it explains.
    """
    try:
        status, out = run()
    except Exception as error:  # noqa: BLE001 # A failure's read mustn't hide what failed.
        return f"nothing, as reading it failed: {error!r}"
    return failure_read(status, out, seconds)
