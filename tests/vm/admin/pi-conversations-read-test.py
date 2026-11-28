"""What the pi host VM test's failures say they read (pi_conversations_read.py), with no machine.

A failed run keeps only its message, so what the host's unit and journals said has to reach it
whole and readable.
"""

from __future__ import annotations

from pi_conversations_read import failure_read

# What coreutils' timeout exits with when its command runs out of time, written out here rather
# than read from the module, so a wrong constant there fails these cases.
TIMEOUT_EXIT = 124


def test_a_read_gives_its_lines_as_one() -> None:
    said = failure_read(0, "first\n\n  \nsecond\n", 30)
    assert said == "first | second", f"blank lines dropped and the rest joined, not {said!r}"


def test_a_read_that_printed_nothing_says_so() -> None:
    said = failure_read(0, "\n", 30)
    assert said == "nothing at all", f"an empty read says so, not {said!r}"


def test_a_read_that_failed_says_its_exit_and_what_it_said() -> None:
    said = failure_read(1, "No such unit\n", 30)
    assert said == "nothing, as it exited 1: No such unit", f"its exit and its words, not {said!r}"
    silent = failure_read(3, "", 30)
    assert silent == "nothing, as it exited 3: saying nothing", f"its exit alone, not {silent!r}"


def test_a_read_that_ran_out_of_time_says_so_with_its_own_seconds() -> None:
    said = failure_read(TIMEOUT_EXIT, "partway\n", 45)
    assert said == "nothing whole, as it ran out of its 45 seconds: partway", (
        f"its time and what it had, not {said!r}"
    )
    silent = failure_read(TIMEOUT_EXIT, "", 7)
    assert silent == "nothing whole, as it ran out of its 7 seconds: saying nothing", (
        f"its time alone, not {silent!r}"
    )


def test_a_read_escapes_only_what_a_terminal_would_act_on() -> None:
    said = failure_read(0, "\x1b[31mred\x1b[0m\tdone, café · ok\n", 30)
    assert said == "\\x1b[31mred\\x1b[0m\\tdone, café · ok", (
        f"control codes escaped and readable words kept as they are, not {said!r}"
    )


def test_a_read_cuts_lines_only_at_a_newline() -> None:
    said = failure_read(0, "a\rb\x0bc\u2028d\x85e\nf\n", 30)
    assert said == "a\\rb\\x0bc\\u2028d\\x85e | f", (
        f"every other break written as its escape, within its line, not {said!r}"
    )
