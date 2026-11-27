"""What the pi host VM test's failures say they read (pi_conversations_read.py), with no machine.

A failed run keeps only its message, so what the host's unit and journals said has to reach it
whole and readable.
"""

from __future__ import annotations

from pi_conversations_read import failure_read, read_for_failure

# What coreutils' timeout exits with when its command runs out of time, written out here rather
# than read from the module, so a wrong constant there fails these cases. That the driver gives
# it is the driver's own: nixos/lib/test-driver's _execute runs a command under `timeout` and
# returns ${PIPESTATUS[0]}, which these cases can't reach.
TIMEOUT_EXIT = 124


def test_a_read_gives_its_lines_as_one() -> None:
    said = failure_read(0, "first\n\n  \nsecond\n", 30)
    assert said == "first | second", f"blank lines dropped and the rest joined, not {said!r}"


def test_a_line_holding_a_control_code_alone_is_kept() -> None:
    said = failure_read(0, "a\n\r\n\x0b\nb\n", 30)
    assert said == "a | \\r | \\x0b | b", f"lines of control codes kept as escapes, not {said!r}"


def test_a_read_quotes_a_backslash_and_a_pipe_so_it_reads_back_one_way() -> None:
    said = failure_read(0, "C:\\x1b | next\n", 30)
    assert said == "C:\\\\x1b \\| next", (
        f"a host's backslash and pipe can't pass for an escape or a break, not {said!r}"
    )


def test_a_read_that_raised_says_so() -> None:
    def gone() -> tuple[int, str]:
        message = "the shell went away"
        raise OSError(message)

    said = read_for_failure(gone, 30)
    assert said == "nothing, as reading it failed: OSError('the shell went away')", (
        f"what was raised, not {said!r}"
    )
    read = read_for_failure(lambda: (0, "ActiveState=active\n"), 30)
    assert read == "ActiveState=active", (
        f"a read that ran is said as failure_read says it, not {read!r}"
    )


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
