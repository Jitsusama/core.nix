"""Dictation's chord held and tapped on a stand-in keyboard, heard by an account.

tests/dictation-chord.nix runs this, and its opening comment says what the test proves.
run-subtests cuts the script into subtests at each top-level `with subtest(`, so every
subtest stays at the top level under its own name, and what sits outside them, the boot,
the listener and the helpers, runs whichever subtests are chosen.
"""

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest, t

heard = "/home/alice/heard"

machine.wait_for_unit("dictation-chord.service")
machine.wait_for_file("/run/dictation-chord/socket")
# systemd-run finds socat itself, as a transient unit's own PATH may not.
machine.succeed(
    "systemd-run --uid=alice --unit=listener"
    f" socat -u UNIX-CONNECT:/run/dictation-chord/socket CREATE:{heard}"
)
machine.wait_for_file(heard, timeout=30)
machine.succeed("systemctl is-active listener.service")


def press(*steps: str) -> list[str]:
    """Press the steps on a new stand-in keyboard and give the lines heard since."""
    before = machine.succeed(f"cat {heard}").splitlines()
    machine.succeed(f"chord-keys {' '.join(steps)}")
    return machine.succeed(f"cat {heard}").splitlines()[len(before) :]


def held(lines: list[str]) -> int:
    """Give the milliseconds the one line heard says the chord was held."""
    t.assertEqual(len(lines), 1, lines)
    word, ms = lines[0].split()
    t.assertEqual(word, "up")
    return int(ms)


with subtest("the account can't read the keyboard itself"):
    machine.succeed("systemd-run --unit=keyboard chord-keys 5000")
    machine.wait_until_succeeds("grep -l 'stand-in keyboard' /sys/class/input/event*/device/name")
    machine.fail("su alice -c 'cat /dev/input/event* > /dev/null'")

with subtest("a held chord says how long it was held"):
    ms = held(press("leftmeta+", "d+", "700", "d-", "leftmeta-"))
    t.assertGreaterEqual(ms, 650)
    t.assertLess(ms, 1500)

with subtest("letting go of Super first ends it too"):
    ms = held(press("leftmeta+", "d+", "500", "leftmeta-", "d-"))
    t.assertGreaterEqual(ms, 450)

with subtest("a tap says it was short"):
    t.assertLess(held(press("leftmeta+", "d+", "d-", "leftmeta-")), 400)

with subtest("D alone, another key, or Super after D says nothing"):
    t.assertEqual(press("d+", "300", "d-"), [])
    t.assertEqual(press("leftmeta+", "a+", "300", "a-", "leftmeta-"), [])
    t.assertEqual(press("d+", "leftmeta+", "300", "leftmeta-", "d-"), [])

with subtest("a keyboard unplugged mid-chord ends it once, and the next is heard afresh"):
    # The stand-in keyboard goes when chord-keys exits, half a second after
    # its last step, with both keys still down, as a dock's keyboard does
    # when the cable comes out during dictation. What's heard then comes as
    # the keyboard is taken down, after chord-keys has returned, so the line
    # is waited for, then a moment more for a second one that mustn't come.
    # The kernel lets a vanishing keyboard's keys go before its readers see
    # it gone, so the chord most often ends on those key-ups, and otherwise
    # on the daemon's own letting go when the read fails; either way, once.
    before = len(machine.succeed(f"cat {heard}").splitlines())
    machine.succeed("chord-keys leftmeta+ d+ 600")
    machine.wait_until_succeeds(f"test $(wc -l < {heard}) -gt {before}", timeout=10)
    machine.sleep(1)
    ms = held(machine.succeed(f"cat {heard}").splitlines()[before:])
    t.assertGreaterEqual(ms, 550)
    t.assertLess(held(press("leftmeta+", "d+", "d-", "leftmeta-")), 400)
