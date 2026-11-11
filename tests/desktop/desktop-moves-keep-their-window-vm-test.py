"""The desktop test's moves keep their window area.

A session the launcher holds moves into pi's host in its own window, as the harness moves one
(its mesh-workflow's in-place.ts and scripts/host-join.ts): pi ends naming the join, the join
names the host's viewer, or plain pi when the host won't take it, and each is a start of the
same holder on the same terminal. A viewer whose host releases the session ends as attach does
then, and what runs in the window carries on as plain pi, as the rollback asks. Joel was told
the windows stay where they are (OSPHT4, 11 November), so the window keeps its id, workspace,
column and size, takes no focus, never changes on his screen, and loses none of his keys; the
node gives the session that window, so the launcher opens no second viewer and Go To goes to
it. The programs are tests/pi-stand-in.sh, whose in-place modes stand in for pi, the join and
the viewer; how the real pi goes quiet and the real join decides are proven in the harness's
own tests.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import datetime as dt
import json
import shlex
import tarfile
import time
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import (
    PI_PRESENCE,
    SHELL_STARTS,
    focus_given,
    watch_the_focus,
    window_box,
)
from PIL import Image, ImageChops

if TYPE_CHECKING:
    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine

# Where the move's frames are recorded, what the recorder says, and the frames taken out.
RECORDING = "/run/user/1000/move.mkv"
RECORDED = "/run/user/1000/move.log"
FRAMES = "/run/user/1000/move-frames"
# How much lighter or darker a recorded pixel may be than the same pixel still, from the
# encoder alone, and the share of a window's pixels past it (one in this many) that is a change
# Joel would see, as desktop-launcher-finds-sessions judges a spawn's.
ENCODER_NOISE = 40
CHANGED_SHARE = 500
# Where the realm's holds keep their records, the realm being the one tests/desktop.nix gives.
HOLDS = "$XDG_RUNTIME_DIR/mycelium/harbour.held"
# How long a step of the move may take on a loaded builder.
STEP = dt.timedelta(seconds=60)
# The words Joel types on past the window's last start, so keys typed into it are seen too.
TYPED_PAST = 20


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


def test_a_session_moving_into_its_host_keeps_its_window_and_every_key(
    desktop: Desktop,
) -> None:
    """A held session moves into its host in its window, and Go To goes to that window.

    The launcher kept the window's place first, as the cutover asks it, and the session runs
    hosted once it moves, which is when a viewer would open there; none does, since the node
    says the window is the session's.
    """
    machine = desktop.machine
    session = "01d1-moves"
    try:
        window = opened_held(desktop, session)
        before = go_to_from_elsewhere(desktop, session, window["id"])
        box, watching = watched_while_typing(desktop, session, window["id"])
        kept = desktop.as_account(f"qs ipc call sessions keepPlace {session}").strip()
        assert kept != "", "the launcher kept no place"
        moved_to_the_join(desktop, session)
        sleeper = desktop.as_account(
            "setsid -f sleep 601 > /dev/null 2>&1; sleep 0.2; pgrep -n -f 'sleep 60[1]'"
        ).strip()
        says_hosted(desktop, session, int(sleeper))
        desktop.as_account(f"touch ~/hosted-{session}")
        started_as(desktop, session, "attach")
        typed_past(desktop, session)
        changed = frames_changed(desktop, box)
        typed = typed_by(desktop, session)
        assert typed == [f"w{n}" for n in range(1, len(typed) + 1)], typed
        assert_unmoved(desktop, window, watching)
        assert changed == [], f"the window Joel types in changed in frames {changed}"
        assert programs_for(desktop, session) == 1, "more than one program runs for it"
        # The node gives the hosted session the window its hold is in.
        row = node_row(desktop, session)
        assert row.get("held") == holding(desktop, session), row
        assert row.get("window") is not None, row
        after = go_to_from_elsewhere(desktop, session, window["id"])
        viewers = [one for one in desktop.windows() if one["app_id"] == f"pi-hosted.{session}"]
        assert viewers == [], f"a second window opened for the session: {viewers}"
        machine.log(f"Go To took {before:.2f} s before the move and {after:.2f} s after")
    finally:
        cleaned(desktop, session)


def test_a_session_its_host_wont_take_carries_on_as_plain_pi_in_its_window(
    desktop: Desktop,
) -> None:
    """A move the host refuses ends in plain pi on the session's log, in the same window.

    Only that one program runs for the session then, so its log has one writer.
    """
    session = "01d1-refused"
    try:
        window = opened_held(desktop, session)
        box, watching = watched_while_typing(desktop, session, window["id"])
        moved_to_the_join(desktop, session)
        desktop.as_account(f"touch ~/refused-{session}")
        started_as(desktop, session, "again")
        typed_past(desktop, session)
        changed = frames_changed(desktop, box)
        typed = typed_by(desktop, session)
        assert typed == [f"w{n}" for n in range(1, len(typed) + 1)], typed
        assert_unmoved(desktop, window, watching)
        assert changed == [], f"the window Joel types in changed in frames {changed}"
        assert programs_for(desktop, session) == 1, "more than one program runs for it"
    finally:
        cleaned(desktop, session)


def test_a_session_its_host_releases_carries_on_as_plain_pi_in_its_window(
    desktop: Desktop,
) -> None:
    """A hosted session the host lets go of carries on as plain pi on its log, in its window.

    Its viewer ends as attach does on a release, with 3, and only then does what runs in the
    window start the join again, which names plain pi: one program for the session, and no
    window opened or closed, as plan item 11's rollback needs of every window Joel placed.
    """
    session = "01d1-released"
    try:
        window = opened_held(desktop, session)
        moved_to_the_join(desktop, session)
        desktop.as_account(f"touch ~/hosted-{session}")
        started_as(desktop, session, "attach")
        box, watching = watched_while_typing(desktop, session, window["id"])
        desktop.as_account(f"touch ~/released-{session}")
        started_as(desktop, session, "again")
        typed_past(desktop, session)
        changed = frames_changed(desktop, box)
        typed = typed_by(desktop, session)
        assert typed == [f"w{n}" for n in range(1, len(typed) + 1)], typed
        assert_unmoved(desktop, window, watching)
        assert changed == [], f"the window Joel types in changed in frames {changed}"
        assert programs_for(desktop, session) == 1, "more than one program runs for it"
        record = json.loads(desktop.as_account(f"cat {HOLDS}/{holding(desktop, session)}.json"))
        assert record["command"][:2] == ["pi", "--session"], record
    finally:
        cleaned(desktop, session)


def opened_held(desktop: Desktop, session: str) -> Json:
    """Reopen a session as the launcher does, held, focused, and give its window once drawn."""
    machine = desktop.machine
    folder = f"/home/jitsusama/{session}"
    desktop.as_account(
        f"touch $XDG_RUNTIME_DIR/pi-moves-in-place && mkdir -p {folder} {PI_PRESENCE}"
        f" && rm -f ~/typed-{session} ~/move-{session} ~/hosted-{session} ~/refused-{session}"
        f" ~/released-{session} $XDG_RUNTIME_DIR/moves-{session}"
    )
    handed = {"sessions": [{"id": session, "cwd": folder, "focus": True}]}
    answer = desktop.as_account(
        f"qs ipc call sessions reopen {shlex.quote(json.dumps(handed))}"
    ).strip()
    assert answer == "taken", answer
    machine.wait_until_succeeds(
        desktop.in_session(f"test -s $XDG_RUNTIME_DIR/moves-{session}"), timeout=SHELL_STARTS
    )
    machine.wait_until_succeeds(
        desktop.in_session(f"niri msg --json windows | grep -q 'pi --session {session}'"),
        timeout=STEP,
    )
    pid, wake = desktop.as_account(f"cat $XDG_RUNTIME_DIR/moves-{session}").split()
    assert wake.endswith(".wake"), f"the session's pi isn't held: {wake}"
    says(desktop, session, {"owner": owner_of(desktop, int(pid)), "wake": wake})
    # Go To goes to a session's window only once the node has joined the two.
    seen: list[dict[str, Any]] = []

    def windowed(_last: bool) -> bool:
        try:
            seen[:] = [node_row(desktop, session)]
        except AssertionError:
            # The node hasn't read the presence yet; the next look may find it.
            return False
        return bool(seen[0].get("held") and seen[0].get("window"))

    try:
        desktop_session.retry(windowed, timeout=STEP)
    except Exception as failed:
        holds = desktop.as_account(f"cat {HOLDS}/*.json || true")
        msg = f"the node never gave the session its window: {seen} with holds {holds}"
        raise AssertionError(msg) from failed
    return next(one for one in desktop.windows() if one["title"] == f"pi --session {session}")


def go_to_from_elsewhere(desktop: Desktop, session: str, window: int) -> float:
    """Go to the session from another window with the launcher's Go To, giving how long it took.

    That's from Return to niri focusing the session's window, which is the only window Go To
    may focus.
    """
    machine = desktop.machine
    titles = {one["title"] for one in desktop.windows()}
    if "Elsewhere" not in titles:
        desktop.opened("Elsewhere")
    desktop.as_account(
        f"niri msg action focus-window --id {desktop.window_titled('Elsewhere')['id']}"
    )
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in(session.split("-")[1], selecting=title_of(session))
    # Return talks to a session that's listening, as this one is while it waits to move, so Go
    # To is its first act within, offered once the launcher knows its window.
    desktop.follow()
    try:
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Go To")
    except AssertionError as failed:
        msg = (
            f"the launcher offers no Go To: {desktop.showing()['rows']},"
            f" while the node says {node_row(desktop, session)}"
            f" and niri {desktop.as_account('niri msg --json windows')}"
        )
        raise AssertionError(msg) from failed
    began = time.monotonic()
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session(f"niri msg --json focused-window | grep -q '\"id\":{window},'"),
        timeout=dt.timedelta(seconds=10),
    )
    return time.monotonic() - began


def watched_while_typing(
    desktop: Desktop, session: str, window: int
) -> tuple[tuple[int, int, int, int], Json]:
    """Focus the session's window, then watch the focus, type into it and record the screen.

    It gives where the window shows and where it sat, its workspace, column and size, as niri
    lists them before anything moves.
    """
    machine = desktop.machine
    desktop.as_account(f"niri msg action focus-window --id {window}")
    machine.wait_until_succeeds(
        desktop.in_session(f"niri msg --json focused-window | grep -q '\"id\":{window},'"),
        timeout=dt.timedelta(seconds=10),
    )
    sat = next(one for one in desktop.windows() if one["id"] == window)
    box = window_box(desktop, window)
    watch_the_focus(desktop)
    # Ten numbered words a wtype, so a key that went elsewhere leaves a gap, as in
    # desktop-launcher-finds-sessions.
    typing = (
        "i=0; while [ ! -e ~/typing-done ]; do"
        ' wtype "$(seq -f "w%.0f " $((i+1)) $((i+10)) | tr -d "\\n")"; i=$((i+10));'
        " echo $i > ~/typing-count; done; wtype -k Return"
    )
    desktop.as_account(
        f"rm -f ~/typing-done ~/typing-count; setsid sh -c {shlex.quote(typing)} > /dev/null 2>&1 &"
    )
    machine.wait_until_succeeds(
        desktop.in_session(f"[ -s ~/typed-{session} ]"), timeout=dt.timedelta(seconds=30)
    )
    record_the_screen(desktop)
    return box, sat


def moved_to_the_join(desktop: Desktop, session: str) -> None:
    """Tell the session's pi to move, as the cutover does, and wait for its join to start."""
    desktop.as_account(f"touch ~/move-{session}")
    started_as(desktop, session, "join")
    # The hold's record names the join's command now, as the node matches it by.
    record = json.loads(desktop.as_account(f"cat {HOLDS}/{holding(desktop, session)}.json"))
    assert record["command"][:2] == ["pi", "join"], record


def started_as(desktop: Desktop, session: str, kind: str) -> None:
    """Wait until what runs for the session in its window is the stand-in for `kind`."""
    desktop.machine.wait_until_succeeds(
        desktop.in_session(f"grep -q ' {kind}$' $XDG_RUNTIME_DIR/moves-{session}"), timeout=STEP
    )


def typed_past(desktop: Desktop, session: str) -> None:
    """Type on past what started last, then stop typing and wait for every key to be kept."""
    machine = desktop.machine
    since = int(desktop.as_account("cat ~/typing-count").strip() or "0")
    machine.wait_until_succeeds(
        desktop.in_session(f"[ $(cat ~/typing-count) -ge {since + TYPED_PAST} ]"), timeout=STEP
    )
    desktop.as_account("touch ~/typing-done")
    machine.wait_until_fails("pgrep -f 'typing-don[e]'", timeout=STEP)
    last = int(desktop.as_account("cat ~/typing-count").strip())
    machine.wait_until_succeeds(
        desktop.in_session(f"grep -q 'w{last}' ~/typed-{session}"), timeout=STEP
    )


def programs_for(desktop: Desktop, session: str) -> int:
    """Count the stand-ins running for the session, its holder aside, which name pi bare."""
    # Bracketed, so the pattern doesn't match the shell running it.
    return int(desktop.as_account(f"pgrep -c -f 'bin/p[i] .*{session}' || true").strip())


def typed_by(desktop: Desktop, session: str) -> list[str]:
    """Give the words the session's programs took from its terminal, in order."""
    return desktop.as_account(f"cat ~/typed-{session}").split()


def assert_unmoved(desktop: Desktop, sat: Json, watching: Json) -> None:
    """Assert the window is where it sat, as it was, and niri focused nothing else meanwhile."""
    now = next((one for one in desktop.windows() if one["id"] == sat["id"]), None)
    assert now is not None, f"window {sat['id']} is gone: {desktop.windows()}"
    assert now["workspace_id"] == watching["workspace_id"], now
    assert (
        now["layout"]["pos_in_scrolling_layout"] == watching["layout"]["pos_in_scrolling_layout"]
    ), now
    assert now["layout"]["window_size"] == watching["layout"]["window_size"], now
    windows, workspaces = focus_given(desktop)
    assert set(windows) <= {sat["id"]}, windows
    assert set(workspaces) <= {watching["workspace_id"]}, workspaces


def holding(desktop: Desktop, session: str) -> str:
    """Give the id of the hold whose program is the session's, as its record names it."""
    records = [
        json.loads(desktop.as_account(f"cat {path}"))
        for path in desktop.as_account(f"ls {HOLDS}/*.json").split()
    ]
    ids = [
        str(record["id"])
        for record in records
        if any(session in word for word in record.get("command", []))
    ]
    assert len(ids) == 1, f"holds running {session}: {ids}"
    return ids[0]


def node_row(desktop: Desktop, session: str) -> dict[str, Any]:
    """Give the node's row for the session, as `mycelium sessions` lists it."""
    rows: list[dict[str, Any]] = []
    for line in desktop.as_account("mycelium --realm harbour sessions").splitlines():
        said = json.loads(line)
        ok = said.get("Ok", said)
        rows += ok.get("Sessions", []) if isinstance(ok, dict) else []
    found = [row for row in rows if row.get("presence", {}).get("session") == session]
    assert found, f"the node lists no {session}: {rows}"
    return found[0]


def title_of(session: str) -> str:
    """Give the title the session's presence says, as its launcher row shows it."""
    return f"Moves in place, {session.split('-')[1]}"


def owner_of(desktop: Desktop, pid: int) -> Json:
    """Give a presence's owner for a process, as pi writes its own."""
    ticks = desktop.as_account(f"awk '{{sub(/.*\\) /, \"\"); print $20}}' /proc/{pid}/stat")
    return {
        "pid": pid,
        "bootId": desktop.as_account("cat /proc/sys/kernel/random/boot_id").strip(),
        "startTicks": int(ticks),
        "startedAt": int(desktop.as_account("date +%s%3N")),
    }


def says(desktop: Desktop, session: str, said: Json) -> None:
    """Replace the session's presence with this, by rename, as pi does."""
    whole = {
        "session": session,
        "addresses": [session.split("-")[1].upper()],
        "title": title_of(session),
        "at": dt.datetime.now(dt.UTC).isoformat(),
        **said,
    }
    name = shlex.quote(json.dumps(whole))
    desktop.as_account(
        f"echo {name} > {PI_PRESENCE}/.{session} && mv {PI_PRESENCE}/.{session}"
        f" {PI_PRESENCE}/{session}.json"
    )


def says_hosted(desktop: Desktop, session: str, host: int) -> None:
    """Say the session runs in the realm's attended host, as the host's presence for it does."""
    says(
        desktop,
        session,
        {
            "owner": owner_of(desktop, host),
            "hosted": {"host": "harbour-attended", "conversation": session},
        },
    )


def cleaned(desktop: Desktop, session: str) -> None:
    """Stop the typing, the recorder and the session's programs, and take away what they left.

    The launcher closes too, so a test failing with it open fails alone.
    """
    desktop.launcher_left_closed()
    desktop.as_account(
        f"touch ~/typing-done ~/move-{session} ~/refused-{session} ~/released-{session};"
        " pkill -f 'event-strea[m]';"
        " pkill -INT -x wf-recorder; pkill -f 'sleep 60[1]';"
        # Bracketed, so the pattern doesn't match the shell running it.
        f" pkill -f '[p]i .*{session}'; rm -f $XDG_RUNTIME_DIR/pi-moves-in-place"
        f" {PI_PRESENCE}/{session}.json $XDG_RUNTIME_DIR/moves-{session}; true"
    )
    desktop.as_account(
        f"rm -rf ~/{session} ~/typed-{session} ~/move-{session} ~/hosted-{session}"
        f" ~/released-{session} ~/refused-{session} ~/typing-done ~/typing-count ~/focus-events"
    )


def record_the_screen(desktop: Desktop) -> None:
    """Record every frame of the focused screen from now on, for frames_changed to read."""
    output = (
        'niri msg --json focused-output | grep -o \'"name":"[^"]*"\' | head -1 | cut -d\'"\' -f4'
    )
    # Every frame, damaged or not (-D), so a still window is a run of equal frames.
    desktop.as_account(
        f'rm -f {RECORDING}; setsid -f wf-recorder -y -D --output "$({output})"'
        f" --file {RECORDING} </dev/null >{RECORDED} 2>&1"
    )
    began, _ = desktop.machine.execute(
        f"timeout 20 sh -c 'until grep -q \"selected region\" {RECORDED}; do sleep 0.1; done'"
    )
    assert began == 0, f"wf-recorder never began: {desktop.machine.execute(f'cat {RECORDED}')[1]}"


def frames_changed(desktop: Desktop, box: tuple[int, int, int, int]) -> list[int]:
    """Stop recording, and give each frame in which the box differs from the first.

    A frame differs where over a five-hundredth of its pixels are lighter or darker by more
    than the encoder's own noise. The first that does is kept beside the test's output.
    """
    machine = desktop.machine
    machine.succeed("pkill -INT -x wf-recorder")
    machine.wait_until_fails("pgrep -x wf-recorder", timeout=dt.timedelta(seconds=30))
    x, y, width, height = box
    ffmpeg = f"{desktop.values['ffmpeg_headless']}/bin/ffmpeg -v error"
    machine.succeed(
        f"rm -rf {FRAMES} && mkdir -p {FRAMES} && {ffmpeg} -i {RECORDING}"
        f" -vf crop={width}:{height}:{x}:{y},fps=30 {FRAMES}/%05d.png"
        f" && tar -C {FRAMES} -cf {FRAMES}.tar ."
    )
    machine.copy_from_machine(f"{FRAMES}.tar")
    kept = machine.out_dir / "move-frames.tar"
    frames = []
    with tarfile.open(kept) as tar:
        for member in sorted(tar.getmembers(), key=lambda member: member.name):
            read = tar.extractfile(member)
            if read is not None:
                frames.append(Image.open(read).convert("L"))
    kept.unlink()
    assert len(frames) > 1, f"the recording held {len(frames)} frames"
    changed: list[int] = []
    for at, frame in enumerate(frames):
        apart = ImageChops.difference(frame, frames[0]).point(
            lambda light: 255 * (light > ENCODER_NOISE)
        )
        if sum(apart.histogram()[255:]) * CHANGED_SHARE > frame.width * frame.height:
            if not changed:
                frame.save(machine.out_dir / "move-changed.png")
            changed.append(at)
    return changed
