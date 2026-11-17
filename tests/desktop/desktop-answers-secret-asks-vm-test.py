"""The desktop test's answers secret asks area.

Here the passwords daemon the shell runs answers an agent's secret ask, which the launcher draws
in Asks: Joel's own yes releases the value into the one command asked for, his r refuses it, and
a yes walked through the launcher's IPC, which any process running as him reaches, releases
nothing.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. It proves the wiring alone, one path through each
connection: how long a line must be shown, a digest that changed, a command that ends first and
what the fence hides are the passwords daemon's own tests' (modules/passwords/tests/asks), and
what Asks sends for each press is asks-over's (tests/asks-over.qml). Every test asks for the
desktop, started once a run (desktop_session.py), and gives each ask a why of its own, which its
line's drawer says, so a line answered earlier is never taken for it. Each test's asks are taken
away after it, passed or failed, so any test runs alone.
"""

from __future__ import annotations

import datetime as dt
import hashlib
import json
import shlex
import time
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from desktop_session import Desktop

# The one reference the machine lists (tests/desktop.nix's released), and the stand-in value op
# gives for it (desktop/op-stand-in.sh).
RELEASED = "op://Keeping/Lighthouse Console/password"
STAND_IN = "stand-in-password"
# Where agents put their asks up, and the daemon its answers.
ASKS = "$XDG_RUNTIME_DIR/secret-asks"
# The fence's list of what it hides, which the daemon reads at every ask and refuses every ask
# without, from the harness's install, which no VM has.
FENCE_LIST = (
    "~/.local/share/pi-packages/github.com/Jitsusama/agentic-harness.pi/current/data"
    "/fence-hidden.json"
)
# The tree the ask's command runs in, the one place besides the caches it may write.
TREE = "~/asked"
# Past the key language's wait (300 ms) and the daemon's least time a line is shown (500 ms), so
# a second Return is neither taken for the first doubled nor refused as unread.
PAST_THE_READING = 1.0


@pytest.fixture
def asking(desktop: Desktop) -> Iterator[None]:
    """Give the daemon the fence's list and the ask a tree; after, take every ask away.

    An ask taken away is let go (modules/secret-asks/tests/daemon.rs), so one a failed test left
    waiting can't be answered by the next test's keys.
    """
    fence = json.dumps({"version": 1, "home": [], "sshKept": [], "npmrc": None})
    desktop.as_account(
        f"mkdir -p $(dirname {FENCE_LIST}) {TREE} && printf %s {shlex.quote(fence)} > {FENCE_LIST}"
        f" && rm -f {TREE}/released"
    )
    yield
    if desktop.showing()["open"]:
        desktop.close_launcher()
    # find rather than a glob, since the account's shell is zsh, where a glob matching nothing
    # is an error, and a test that saw its ask answered leaves none.
    desktop.as_account(f"find {ASKS} -maxdepth 1 -name '*.ask.json' -delete")


pytestmark = pytest.mark.usefixtures("asking")


def put_up(desktop: Desktop, ask: str, why: str) -> None:
    """Put an ask up as the harness does, written aside and renamed into place.

    Its command writes the digest of the value it's given into its tree, so the test sees it
    arrive, and prints the value itself, so the answer's output shows it masked.
    """
    asked_ms = int(desktop.as_account("date +%s%3N").strip())
    command = 'printf %s "$GIVEN" | sha256sum > released && printf %s "$GIVEN"'
    said = json.dumps(
        {
            "version": 1,
            "id": ask,
            "asked_ms": asked_ms,
            "session": "TESTER",
            "quest": None,
            "item": RELEASED,
            "why": why,
            "argv": ["/run/current-system/sw/bin/bash", "-c", command],
            "tree": desktop.as_account(f"realpath {TREE}").strip(),
            "given": "GIVEN",
            "minutes": 1,
        }
    )
    desktop.as_account(
        f"mkdir -p {ASKS} && printf %s {shlex.quote(said)} > {ASKS}/.{ask}.writing"
        f" && mv {ASKS}/.{ask}.writing {ASKS}/{ask}.ask.json"
    )


def answered(desktop: Desktop, ask: str) -> dict[str, object]:
    """Give an ask's answer, once the daemon has written it."""
    path = f"{ASKS}/{ask}.answer.json"
    desktop.machine.wait_until_succeeds(
        desktop.in_session(f"test -e {path}"), timeout=dt.timedelta(seconds=30)
    )
    answer: dict[str, object] = json.loads(desktop.as_account(f"cat {path}"))
    return answer


def selects(shown: dict[str, object], why: str) -> bool:
    """Say whether the launcher's selected line is the ask with this why."""
    selected = shown["selected"] or {}
    assert isinstance(selected, dict), f"the selected line isn't a row: {selected!r}"
    return bool(selected.get("drawer", {}).get("body") == why)


def asks_open_on(desktop: Desktop, ask: str, why: str) -> None:
    """Open the launcher on Asks, the ask's line selected, or say what became of the ask."""
    desktop.as_account("qs ipc call launcher collection asks")
    desktop.launcher_ready()
    try:
        desktop.launcher_until(lambda shown: selects(shown, why))
    except AssertionError as unselected:
        answer = desktop.as_account(f"cat {ASKS}/{ask}.answer.json 2>/dev/null || true")
        message = f"the ask's line was never selected; its answer: {answer or 'none'}"
        raise AssertionError(message) from unselected


def refuse_on(desktop: Desktop, why: str) -> None:
    """Press r on the ask's line, as Joel does: in normal, since on the find it's typed."""
    desktop.machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["mode"] == "normal" and selects(shown, why))
    desktop.machine.send_chars("r")


def read_nowhere(desktop: Desktop) -> None:
    """Assert the value is in no journal, the system's or any account's, and no file of the tree.

    The journal is flushed first, so what the daemon said just now is read too.
    """
    desktop.machine.succeed("journalctl --sync")
    found = desktop.machine.succeed(
        f"journalctl --no-pager -o export | grep -a -c -F {shlex.quote(STAND_IN)} || true"
    )
    assert found.strip() == "0", f"the journal holds the value {found.strip()} times"
    desktop.as_account(f"! grep -r -a -q -F {shlex.quote(STAND_IN)} {TREE}")


def test_joels_yes_releases_the_value_into_the_one_command(desktop: Desktop) -> None:
    """Joel's yes, Return twice on the ask's line, releases the value into its command alone."""
    ask = "0123456789abcdef0123456789abcdef"
    why = "To fire the kiln."
    put_up(desktop, ask, why)
    asks_open_on(desktop, ask, why)
    machine = desktop.machine
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["returned"].get("act") == "armed")
    time.sleep(PAST_THE_READING)
    machine.send_key("ret")
    answer = answered(desktop, ask)
    assert (answer["state"], answer["exit"]) == ("ran", 0), answer
    given = desktop.as_account(f"cat {TREE}/released").split()[0]
    assert given == hashlib.sha256(STAND_IN.encode()).hexdigest(), "the command wasn't given it"
    assert answer["output"], f"the command's output isn't in its answer: {answer}"
    assert STAND_IN not in json.dumps(answer), answer
    read_nowhere(desktop)


def test_r_refuses_an_ask(desktop: Desktop) -> None:
    """R on an ask's line refuses it, releasing nothing."""
    ask = "1123456789abcdef0123456789abcdef"
    why = "To glaze the pots."
    put_up(desktop, ask, why)
    asks_open_on(desktop, ask, why)
    refuse_on(desktop, why)
    answer = answered(desktop, ask)
    assert (answer["state"], answer["why"]) == ("refused", "refused by Joel"), answer
    desktop.as_account(f"! test -e {TREE}/released")


def test_a_yes_walked_through_the_ipc_releases_nothing(desktop: Desktop) -> None:
    """A yes walked through the launcher's IPC, as any process running as Joel may, is withheld.

    Only his own key press on the drawn line grants an ask, so the ask still waits after it. The
    line stays drawn past the time a yes needs, so only the walk's being no key press of his
    withholds it.
    """
    ask = "2123456789abcdef0123456789abcdef"
    why = "To light the forge."
    put_up(desktop, ask, why)
    asks_open_on(desktop, ask, why)
    time.sleep(PAST_THE_READING)
    since = desktop.mark()
    walked = json.dumps({"steps": ["Passwords", "Asks", RELEASED, "Release It"], "sure": True})
    said = desktop.as_account(f"qs ipc call launcher doing {shlex.quote(walked)}")
    answer = json.loads(said)
    assert "missing" not in answer, f"the walk found no Release It: {said}"
    # The agent that walked is told no, so it doesn't wait out the ask.
    assert "done" not in answer, f"the walk was answered done: {said}"
    assert answer.get("refused") == "not without your own key press", (
        f"the walk wasn't told why: {said}"
    )
    # The shell's own line, so the withholding was the shell's, not the daemon's.
    desktop.machine.wait_until_succeeds(
        desktop.in_session(
            f"journalctl --user -u quickshell --no-pager -o cat --after-cursor={shlex.quote(since)}"
            " | grep -q 'verb:release withheld'"
        ),
        timeout=dt.timedelta(seconds=30),
    )
    desktop.as_account(f"test -e {ASKS}/{ask}.ask.json && ! test -e {ASKS}/{ask}.answer.json")
    desktop.as_account(f"! test -e {TREE}/released")


def scene_warnings(desktop: Desktop) -> str:
    """Give each warning Quickshell's scene gave, one a line, or nothing.

    A binding that throws or assigns the wrong type only warns, and only once a surface draws
    it. This belongs in desktop_session.py with the other areas' copies, which wait to move
    there (tests/rules/every-test-function-lives-once-waiting.nix).
    """
    return desktop.as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    ).strip()


def test_quickshell_warned_of_nothing_it_drew_all_along(desktop: Desktop) -> None:
    """Quickshell warned of nothing it drew all along.

    Quickshell warns of a scene only once a surface draws it, so this comes last.
    """
    warned = scene_warnings(desktop)
    assert warned == "", f"Quickshell warned of a scene: {warned}"
