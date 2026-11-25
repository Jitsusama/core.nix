"""The desktop test's answers secret asks area.

Here the passwords daemon the shell runs answers an agent's secret ask, which the launcher draws
in Asks: Joel's own yes releases the value into the one command asked for, his r refuses it, and
a yes walked through the launcher's IPC, which any process running as him reaches, releases
nothing, as does one after an agent's run or walk onto Asks came between his Escape and his move.

tests/desktop/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. It proves the wiring alone, one path through each
connection. How long a line must be shown, a digest that changed, a command that ends first and
what the fence hides are the passwords daemon's own tests' (modules/passwords/tests/asks), which
run a stand-in bubblewrap; this test sends one file it empties, Docker's settings, through the
real one. What Asks sends for each press is asks-over's (modules/quickshell/tests/asks-over.qml).
Every test asks for the desktop, started once a run (desktop_session.py), and gives each ask a why
of its own, which its line's drawer says, so a line answered earlier is never taken for it. Each
test's asks are taken away after it, passed or failed, so any test runs alone.
"""

from __future__ import annotations

import datetime as dt
import hashlib
import json
import re
import shlex
import time
from typing import TYPE_CHECKING

import pytest
import vm_helpers

if TYPE_CHECKING:
    from collections.abc import Iterator

    from desktop_session import Desktop

# The one reference the machine lists (tests/desktop/desktop.nix's released), and the stand-in value
# op gives for it (tests/desktop/stand-ins/op-stand-in.sh).
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
# Docker's settings with a registry's made-up login, which the fence's list empties for the
# command, and the settings saying nothing bound over them.
DOCKER = "~/.docker/config.json"
DOCKER_LOGIN = '{"auths": {"registry.example": {"auth": "ZG9ja2VyLW1hZGUtdXA="}}}'
EMPTIED = "{}"
# Past the key language's wait (300 ms) and the daemon's least time a line is shown (500 ms), so
# a second Return is neither taken for the first doubled nor refused as unread. It's slept, not
# waited for, so past_the_reading paces it by hand, as the driver paces only its own waits.
PAST_THE_READING = 1.0
# How long the daemon may take to write an ask's answer on a quiet host.
ANSWER_WRITTEN = dt.timedelta(seconds=30)


@pytest.fixture
def asking(desktop: Desktop) -> Iterator[None]:
    """Give the daemon the fence's list and the ask a tree; after, take every ask away.

    An ask taken away is let go (modules/secret-asks/tests/daemon.rs), so one a failed test left
    waiting can't be answered by the next test's keys.
    """
    fence = json.dumps(
        {
            "version": 2,
            "home": [],
            "sshKept": [],
            "npmrc": None,
            "emptied": [{"path": ".docker/config.json", "when": ["auths"]}],
        }
    )
    desktop.as_account(
        f"mkdir -p $(dirname {FENCE_LIST}) $(dirname {DOCKER}) {TREE}"
        f" && printf %s {shlex.quote(fence)} > {FENCE_LIST}"
        f" && printf %s {shlex.quote(DOCKER_LOGIN)} > {DOCKER}"
        f" && rm -f {TREE}/released {TREE}/docker"
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
    arrive, and prints the value itself, so the answer's output shows it masked. It copies
    Docker's settings as it reads them there too, first and whatever comes of it, so a test sees
    what the fence left it of them.
    """
    asked_ms = int(desktop.as_account("date +%s%3N").strip())
    command = (
        f'cat {DOCKER} > docker; printf %s "$GIVEN" | sha256sum > released && printf %s "$GIVEN"'
    )
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


def answered(desktop: Desktop, ask: str, since: str | None = None) -> dict[str, object]:
    """Give an ask's answer, once the daemon has written it.

    Given the mark from before the yes, one never written says what the shell said of that
    yes after it, since a yes refused as unread and a slow daemon both leave no answer. The
    driver's wait is paced already (vm_helpers.pace_the_driver).
    """
    path = f"{ASKS}/{ask}.answer.json"
    try:
        desktop.machine.wait_until_succeeds(
            desktop.in_session(f"test -e {path}"), timeout=ANSWER_WRITTEN
        )
    except Exception as unanswered:
        if since is None:
            raise
        # Its showings hold the ask's row too, so only the shell's words of it are kept. They're
        # sorted here rather than by a pipe, whose status would be its last grep's alone, so a
        # journal that couldn't be read says how it ended rather than reading as nothing said:
        # diagnosis puts that, or what the read raised, on a line of its own opening "(".
        journal = f"journalctl --user -u quickshell -o cat --after-cursor={shlex.quote(since)}"
        # A yes the shell withholds is said by its act's key, verb:release, not the ask's; only
        # this test's ask is waiting, so either names it.
        named = (f"ask:{ask}", "verb:release")
        how_it_ended = ("(un", "(ended with ")
        words = [
            line
            for line in desktop.diagnosis(journal).splitlines()
            if (any(name in line for name in named) and "report[showing " not in line)
            or line.startswith(how_it_ended)
        ]
        said = "\n".join(words)
        never = f"no answer came to {ask}; of it the shell said: {said.strip() or 'nothing'}"
        raise AssertionError(never) from unanswered
    answer: dict[str, object] = json.loads(desktop.as_account(f"cat {path}"))
    return answer


def selects(shown: dict[str, object], why: str) -> bool:
    """Say whether the launcher's selected line is the ask with this why."""
    selected = shown["selected"] or {}
    assert isinstance(selected, dict), f"the selected line isn't a row: {selected!r}"
    return bool(selected.get("drawer", {}).get("body") == why)


def asks_open_on(desktop: Desktop, ask: str, why: str) -> None:
    """Open the launcher on Asks from outside, the ask's line selected, or say what became of it.

    Any process running as Joel may open it so, which times no line until a key of his moves.
    """
    desktop.as_account("qs ipc call launcher collection asks")
    desktop.launcher_ready()
    selected_or_told(desktop, ask, why)


def selected_or_told(desktop: Desktop, ask: str, why: str) -> None:
    """Wait for the ask's line to be selected, or say what became of the ask."""
    try:
        desktop.launcher_until(lambda shown: selects(shown, why))
    except AssertionError as unselected:
        answer = desktop.as_account(f"cat {ASKS}/{ask}.answer.json 2>/dev/null || true")
        message = f"the ask's line was never selected; its answer: {answer or 'none'}"
        raise AssertionError(message) from unselected


def past_the_reading() -> None:
    """Sleep past the key language's wait and the least time shown, paced as a wait is."""
    time.sleep(PAST_THE_READING * vm_helpers.pace(vm_helpers.busyness()))


def say_yes(desktop: Desktop) -> str:
    """Press Return twice on the selected line, as Joel says yes: the first arms it.

    Gives the shell's journal's mark from just before the second, the yes.
    """
    desktop.machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["returned"].get("act") == "armed")
    past_the_reading()
    since = desktop.mark()
    desktop.machine.send_key("ret")
    return since


def refused_unread(desktop: Desktop, ask: str, since: str) -> None:
    """Assert the ask's yes had no time shown, so the launcher refused it as unread.

    A yes refused as unread is no answer, so the ask waits on for one that's read: the shell
    says why it didn't become, and no answer is written nor anything run.
    """
    desktop.reported(
        f"not become ask:{ask}: ",
        lambda why: why == "press Escape, then j or k on it, and read it before you release it",
        since,
    )
    desktop.as_account(f"test -e {ASKS}/{ask}.ask.json && ! test -e {ASKS}/{ask}.answer.json")
    desktop.as_account(f"! test -e {TREE}/released")


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
    """Joel's yes, Return twice on the ask's line, releases the value into its command alone.

    Bubblewrap itself takes the settings saying nothing bound over Docker's, and the command
    reads those rather than the login.
    """
    ask = "0123456789abcdef0123456789abcdef"
    why = "To fire the kiln."
    put_up(desktop, ask, why)
    asks_open_on(desktop, ask, why)
    # A move of his over the rows times the line, though there's nowhere to move to; on the
    # find, where the launcher opens, an arrow could be a terminal's.
    desktop.machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["mode"] == "normal" and selects(shown, why))
    # The release below proves the j was read as a move, since one not read leaves the line
    # untimed and the yes refused. The first Return waits for the j to be taken: read with it
    # in one turn, the j's move is unknown, and the line's time starts again (Shown.js's
    # keyed). That is likely what failed the net's runs that timed out here, j and Return 10 ms
    # apart, read from the code, since those runs kept no shell journal; answered() names what
    # the shell said should it fail again. Asks reads the j a turn after the launcher counts it
    # (Qt.callLater), long before a Return sent after.
    taken = desktop.showing()["keysTaken"]
    desktop.machine.send_chars("j")
    desktop.launcher_until(lambda shown: shown["keysTaken"] > taken)
    answer = answered(desktop, ask, say_yes(desktop))
    assert (answer["state"], answer["exit"]) == ("ran", 0), answer
    given = desktop.as_account(f"cat {TREE}/released").split()[0]
    assert given == hashlib.sha256(STAND_IN.encode()).hexdigest(), "the command wasn't given it"
    docker = desktop.as_account(f"cat {TREE}/docker").strip()
    assert docker == EMPTIED, f"the command read Docker's settings as {docker!r}"
    assert answer["output"], f"the command's output isn't in its answer: {answer}"
    assert STAND_IN not in json.dumps(answer), answer
    read_nowhere(desktop)


def test_an_asks_opened_from_outside_takes_no_yes_without_a_move(desktop: Desktop) -> None:
    """Asks opened from outside, its line shown past a yes's least time, refuses two Returns.

    His Returns might be meant for a window it took the keys from, so only a move of his times
    the line.
    """
    ask = "3123456789abcdef0123456789abcdef"
    why = "To wind the clock."
    put_up(desktop, ask, why)
    asks_open_on(desktop, ask, why)
    past_the_reading()
    refused_unread(desktop, ask, say_yes(desktop))


def test_r_refuses_an_ask(desktop: Desktop) -> None:
    """R on an ask's line refuses it, releasing nothing, and a Return on it then closes."""
    ask = "1123456789abcdef0123456789abcdef"
    why = "To glaze the pots."
    put_up(desktop, ask, why)
    asks_open_on(desktop, ask, why)
    refuse_on(desktop, why)
    answer = answered(desktop, ask)
    assert (answer["state"], answer["why"]) == ("refused", "refused by Joel"), answer
    desktop.as_account(f"! test -e {TREE}/released")
    # Return on the line once answered only closes, as on any row that only says something,
    # rather than closing by a Return that failed. It waits for the line to read refused, since
    # until then it's held as it was, waiting, where Return arms the yes. Its side leads with
    # what came of it, whatever follows.
    desktop.launcher_until(
        lambda shown: (
            shown["mode"] == "normal"
            and selects(shown, why)
            and shown["selected"]["side"].startswith("refused")
        )
    )
    since = desktop.mark()
    desktop.machine.send_key("ret")
    desktop.launcher_until(lambda shown: not shown["open"])
    # A throw is written in the same turn as the close, but reaches the
    # journal through a stream, so it's synced before it's read; a journal
    # that can't be read fails rather than passing for a clean one. Each
    # read is paced, so a busy builder fails this step by name.
    desktop.machine.succeed("journalctl --sync", timeout=60)
    said = desktop.as_account(
        f"journalctl --user -u quickshell --no-pager -o cat --after-cursor={shlex.quote(since)}"
    )
    # Any error QML writes, not only the one this guards, as Quickshell
    # writes one: "WARN scene: @Launcher.qml[<line>:-1]: TypeError: ...".
    thrown = re.search(r"\b\w*Error: ", said)
    assert thrown is None, f"a Return on the refused line threw: {said}"


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
    past_the_reading()
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


def outside_onto_asks(desktop: Desktop, way: str) -> None:
    """Open Asks as an agent may, by the catalogue's run or a terminal's walk."""
    if way == "run":
        entries = json.loads(desktop.as_account("qs ipc call launcher list"))
        found = [e["id"] for e in entries if e["words"] == "Asks" and e["in"] == "Passwords"]
        assert len(found) == 1, f"the catalogue lists Asks {len(found)} times"
        said = desktop.as_account(f"qs ipc call launcher run {shlex.quote(found[0])}")
    else:
        walked = json.dumps({"steps": ["Passwords", "Asks"]})
        said = desktop.as_account(f"qs ipc call launcher doing {shlex.quote(walked)}")
    assert json.loads(said).get("done") is True, f"the {way} didn't open Asks: {said}"


@pytest.mark.parametrize("way", ["run", "doing"])
def test_an_agents_ask_after_his_escape_leaves_his_move_untimed(desktop: Desktop, way: str) -> None:
    """An agent's run or walk onto Asks after Joel's Escape leaves his next j untimed.

    Either may land the selection on a line he never chose, so his move counts only after an
    Escape of his own that comes after it: two Returns after the j release nothing.
    """
    mark = "1" if way == "run" else "2"
    ask = f"4{mark}23456789abcdef0123456789abcdef"
    why = f"To bank the fire, by {way}."
    # An older ask beneath, waiting, so his move goes off the line and back over waiting asks
    # alone, whatever lines earlier tests left answered further down.
    beneath = f"4{mark}13456789abcdef0123456789abcdef"
    put_up(desktop, beneath, f"To rake the ash, by {way}.")
    put_up(desktop, ask, why)
    asks_open_on(desktop, ask, why)
    desktop.machine.send_key("esc")
    desktop.launcher_until(lambda shown: shown["mode"] == "normal" and selects(shown, why))
    outside_onto_asks(desktop, way)
    # Still normal, so the j and k below move over the rows rather than type into the find.
    desktop.launcher_until(lambda shown: shown["mode"] == "normal" and selects(shown, why))
    desktop.machine.send_chars("j")
    desktop.launcher_until(lambda shown: not selects(shown, why))
    desktop.machine.send_chars("k")
    desktop.launcher_until(lambda shown: selects(shown, why))
    # Past the reading, so a move wrongly taken as his would have timed the line long enough.
    past_the_reading()
    refused_unread(desktop, ask, say_yes(desktop))


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
