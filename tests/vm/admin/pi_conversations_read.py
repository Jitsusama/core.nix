"""What pi-conversations-come-back's failures read from the machine, said on one line.

Only that VM test's driver is given this, so a change here moves no other VM test; its cases are
pi-conversations-read-test.py, which run apart from any machine.
"""

from __future__ import annotations

# What coreutils' timeout exits with when its command ran out of time, named as heard.py in
# tests/desktop names it.
CUT_SHORT = 124


def escaped(line: str) -> str:
    """Give a line with each character a terminal would act on written as its escape."""
    return "".join(c if c.isprintable() else c.encode("unicode_escape").decode() for c in line)


def failure_read(status: int, out: str, seconds: int) -> str:
    """Give what a command a failure reads printed, on one line, or why it can't be read.

    The driver's execute ends a command at its timeout with timeout's own exit, so that is said
    as running out of time. Lines are cut only at a newline, and every other character a
    terminal would act on, a carriage return or a journal's colours among them, is written as
    its escape, so nothing the host wrote can garble the failure that quotes it.
    """
    said = " | ".join(escaped(line) for line in out.split("\n") if line.strip())
    if status == CUT_SHORT:
        return f"nothing whole, as it ran out of its {seconds} seconds: {said or 'saying nothing'}"
    if status != 0:
        return f"nothing, as it exited {status}: {said or 'saying nothing'}"
    return said or "nothing at all"
