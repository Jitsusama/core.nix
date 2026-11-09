"""The desktop test's looks area for Joel's programs' own pages.

Here the pages his programs hold in the launcher are shot at his two scales, on a screen as large
as the laptop's, as the looks area shoots the launcher's own: 1.5, as the HP draws it, and 2, as
the laptop does. Each is walked short, waiting only until the shot shows what it's named for,
since what each does is proven in its own area (VKCF3Z, 2026-11-08: every shot at both scales,
since each has caught something the other didn't). It's an area of its own, rather than more of
looks, since the pyramid keeps a VM test to twenty subtests and about two minutes a subtest
(docs/testing.md), and on its own it runs beside looks rather than after it.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and leaves it as it found it; Passwords' goes last, since its copy stays on
the clipboard until it clears.
"""

from __future__ import annotations

import shlex
from datetime import timedelta
from functools import partial
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest

# pytest finds a fixture by its name in the test's module, so the stand-in Google's is
# imported for Drive's test to ask for.
from desktop_session import LAUNCHER_OPEN, google  # noqa: F401

if TYPE_CHECKING:
    from collections.abc import Iterator

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine

# The scales Joel's screens are drawn at, as niri spells them.
JOELS_SCALES = ("1.5", "2")

# A repository and a machine for kitty's page to list, as launcher-moves-windows gives it; the
# machine is under .test, which nothing in the VM answers, and nothing here runs a row.
REPOSITORY = "tide-tables"
MACHINE = "ferry"
SSH_CONFIG = "Host ferry\n    HostName ferry.harbour.test\n    User deckhand\n    Port 2200\n"

# Where tuple-call keeps the calls that ended, for Tuple's Recent.
TUPLE_KEPT = "~/.local/state/tuple-call/recent.jsonl"

# Windows standing in for Google's programs in Chrome, as launcher-moves-windows' are: a kitty
# given the app id and title Chrome gives Mail's own window, and one given Chrome's app id and a
# Docs tab's title, each by the layer Shortcuts opens on and its shot's name.
GOOGLE_WINDOWS = [
    ("chrome-mail.google.com__-Default", "Inbox - joel@example.com - Gmail", "Mail", "mail"),
    ("google-chrome", "Notes - Google Docs - Google Chrome", "Docs", "docs-tab"),
]


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


@pytest.fixture
def at_each_scale(desktop: Desktop) -> Iterator[list[str]]:
    """Give Joel's scales to set in turn, and put the screen back at 1 afterwards."""
    yield list(JOELS_SCALES)
    # A test that failed with the launcher open leaves it closed for the next.
    if desktop.machine.execute(desktop.in_session(LAUNCHER_OPEN))[0] == 0:
        desktop.close_launcher()
    desktop.as_account("niri msg output winit scale 1")


def at_scale(desktop: Desktop, scale: str) -> str:
    """Set the screen's scale, with the launcher closed, and give it as a shot's name ends."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.as_account(f"niri msg output winit scale {scale}")
    return scale.replace(".", "-")


def drawer_on(title: str, shown: Json) -> bool:
    """Say whether the drawer is out, drawing the thing titled."""
    drawer = shown["drawer"]
    return bool(drawer["out"] is True and drawer["title"] == title)


def figures_on(title: str, shown: Json) -> bool:
    """Say whether the drawer draws every fact of the thing titled, none left for its keys."""
    drawer = shown["drawer"]
    return drawer_on(title, shown) and len(drawer["lines"]["facts"]) == len(drawer["facts"])


def named(name: str, shown: Json) -> bool:
    """Say whether the selected row is the one named."""
    return (shown["selected"] or {}).get("name") == name


@pytest.mark.usefixtures("google")
def test_drives_places_and_recent_are_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot Drive's program's drawer, the program holding its places, and a file in Recent.

    Typed, drive leads with the disk's room, so the selection goes down to the program, whose
    drawer says what its places hold and what its copies kept here take.
    """
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("drive")
        desktop.launcher_until(lambda shown: "Drive" in shown["rows"])
        desktop.down_to("Drive")
        # Each size keeps its unit beside it by a no-break space, as every figure does.
        desktop.launcher_until(
            lambda shown: (
                shown["drawer"]["out"]
                and "Kept Here" in shown["drawer"]["drawnFacts"]
                and "412\u00a0MB of 2.00\u00a0GB" in shown["drawer"]["values"]
            )
        )
        desktop.shoot(f"launcher-drive-at-{at}")
        desktop.into_program()
        desktop.launcher_until(lambda shown: "Starred" in shown["rows"])
        desktop.shoot(f"launcher-drive-places-at-{at}")
        desktop.type_in("recent", selecting="Recent")
        desktop.follow()
        desktop.launcher_until(partial(drawer_on, "menu.pdf"))
        desktop.shoot(f"launcher-drive-recent-at-{at}")
        desktop.close_launcher()


def test_spotify_and_its_podcasts_are_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot Spotify's page signed in, holding Recent and Podcasts, and a show in Podcasts."""
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("spotify")
        desktop.launcher_until(partial(named, "Spotify"))
        desktop.into_program()
        # Spotify is read as it's stepped into, so its lists come once the stand-in answers.
        desktop.launcher_until(lambda shown: {"Recent", "Podcasts"} <= set(shown["rows"]))
        desktop.shoot(f"launcher-spotify-at-{at}")
        desktop.down_to("Podcasts")
        desktop.step_in()
        desktop.launcher_until(lambda shown: shown["rows"][:1] == ["The Show"])
        desktop.shoot(f"launcher-spotify-podcasts-at-{at}")
        desktop.close_launcher()
    # The stand-in spotify-recent notes each run it's asked (tests/desktop.nix).
    desktop.as_account("rm -f ~/asked")


@pytest.fixture
def stopped_part_way(desktop: Desktop) -> Iterator[str]:
    """Give a file mpv stopped part way, as mpv writes it quitting, and take it away after."""
    clip = f"/home/{desktop.account}/Videos/stopped.mkv"
    later = f"/home/{desktop.account}/.local/state/mpv/watch_later"
    # Where mpv stopped in it, named by its path's hash, as opens-files' mpv test writes it.
    kept = f"{later}/$(printf %s {clip} | md5sum | cut -c 1-32 | tr a-f A-F)"
    desktop.as_account(
        f"mkdir -p {later} ~/Videos && mpv --no-config --really-quiet --ovc=ffv1"
        f" 'av://lavfi:testsrc=duration=60:size=64x64:rate=2' --o={clip}"
        f" && printf '# %s\\nstart=12.000000\\nspeed=1.500000\\n' {clip} > {kept}"
    )
    yield clip
    desktop.as_account(f"rm -f {clip} {kept}")


def test_what_mpv_stopped_is_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str], stopped_part_way: str
) -> None:
    """Shoot mpv's Recent on a file it stopped part way, its drawer saying what's left."""
    title = stopped_part_way.rsplit("/", 1)[-1]
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("mpv")
        desktop.launcher_until(partial(named, "mpv Media Player"))
        desktop.into_program()
        desktop.type_in("recent", selecting="Recent")
        desktop.follow()
        desktop.launcher_until(partial(drawer_on, title))
        desktop.shoot(f"launcher-mpv-stopped-at-{at}")
        desktop.close_launcher()


@pytest.fixture
def a_tuple_call_ended(desktop: Desktop) -> Iterator[None]:
    """Have a Tuple call with Ada end, on the stand-in's stream, as takes-calls has one.

    After, tuple-call forgets it once its file goes and it restarts, and Tuple starts again
    where it ran.
    """
    tuple_was = desktop.machine.execute(desktop.in_session("systemctl --user is-active tuple"))
    desktop.tuple_follows()
    now = int(desktop.as_account("date +%s%3N"))
    connected = {"connection_state": "connected", "audio_enabled": True}
    ada = connected | {"id": 7, "full_name": "Ada Brightwater", "email": "ada@example.com"}
    call = {
        "id": "a-call",
        "local": connected | {"is_current_user": True},
        "participants": [ada],
        "started_at": now - 9 * 60 * 1000,
    }
    with desktop.told("drawn margin"):
        desktop.tuple_streams(call)
    desktop.tuple_streams(None)
    desktop.machine.wait_until_succeeds(
        desktop.in_session(f"grep -q Brightwater {TUPLE_KEPT}"), timeout=timedelta(seconds=10)
    )
    yield
    desktop.as_account(
        f"rm -f {TUPLE_KEPT} && systemctl --user stop tuple-answers"
        " && systemctl --user restart tuple-call"
    )
    if tuple_was[0] == 0:
        desktop.as_account("systemctl --user start tuple")


@pytest.mark.usefixtures("a_tuple_call_ended")
def test_tuples_recent_is_shot_at_each_scale(desktop: Desktop, at_each_scale: list[str]) -> None:
    """Shoot Tuple's Recent on the call that ended, its drawer out."""
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("tuple")
        desktop.launcher_until(partial(named, "Tuple"))
        desktop.into_program()
        desktop.launcher_until(lambda shown: "Recent" in shown["rows"])
        desktop.down_to("Recent")
        desktop.step_in()
        desktop.launcher_until(lambda shown: "Ada Brightwater" in shown["rows"])
        desktop.down_to("Ada Brightwater")
        desktop.launcher_until(partial(drawer_on, "Ada Brightwater"))
        desktop.shoot(f"launcher-tuple-recent-at-{at}")
        desktop.close_launcher()


def test_the_vms_are_shot_at_each_scale(desktop: Desktop, at_each_scale: list[str]) -> None:
    """Shoot Programs' VMs on the stand-in virsh's web, shut off, its drawer out."""
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("vms")
        desktop.launcher_until(partial(named, "VMs"))
        desktop.into_program()
        # Every figure fits, none left for its keys; its UUID and security label are
        # identifiers, so they're whole on as many lines as they take, broken at their hyphens,
        # rather than one word running past the drawer's edge, which the shot shows. At scale
        # 2 that takes the line its note on what's known only while it runs had, so the note
        # is what Ctrl+W l reaches there.
        desktop.launcher_until(partial(figures_on, "web"))
        desktop.shoot(f"launcher-vms-at-{at}")
        desktop.close_launcher()
    # The stand-in virsh notes each run it's asked (tests/desktop.nix).
    desktop.as_account("rm -f $XDG_RUNTIME_DIR/virsh-asked")


@pytest.fixture
def a_repository_and_a_machine(desktop: Desktop) -> Iterator[None]:
    """Give kitty a repository in ~/src and a machine in ~/.ssh/config, and take them after.

    The shell reads kitty's repositories and machines ahead, as the launcher closes, so it's
    opened and closed once they're made.
    """
    desktop.as_account(
        f"git init -q ~/src/{REPOSITORY} && mkdir -p -m 700 ~/.ssh && test ! -e ~/.ssh/config"
        f" && printf %s {shlex.quote(SSH_CONFIG)} > ~/.ssh/config"
    )
    desktop.answer("kitty")
    desktop.close_launcher()
    yield
    desktop.as_account(
        f"rm -rf ~/src/{REPOSITORY} && rm -f ~/.ssh/config && (rmdir ~/.ssh || true)"
    )


@pytest.mark.usefixtures("a_repository_and_a_machine")
def test_kittys_repositories_and_machines_are_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot kitty's New Window in a Repository and SSH to a Machine, each stepped into."""
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.answer("kitty")
        desktop.launcher_until(partial(named, "kitty"))
        desktop.into_program()
        desktop.launcher_until(lambda shown: "SSH to a Machine" in shown["rows"])
        desktop.down_to("New Window in a Repository")
        desktop.step_in()
        desktop.launcher_until(lambda shown: REPOSITORY in shown["rows"])
        desktop.shoot(f"launcher-kitty-repositories-at-{at}")
        desktop.step_out()
        desktop.down_to("SSH to a Machine")
        desktop.step_in()
        desktop.launcher_until(lambda shown: MACHINE in shown["rows"])
        desktop.down_to(MACHINE)
        desktop.launcher_until(partial(drawer_on, MACHINE))
        desktop.shoot(f"launcher-kitty-ssh-at-{at}")
        desktop.close_launcher()


def on_layer(layer: str, shown: Json) -> bool:
    """Say whether Shortcuts holds the layer named, as its path ends."""
    return cast("list[str]", shown["path"])[-1:] == [layer]


def test_shortcuts_over_google_are_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot Shortcuts opened by Super+Shift+/ over Mail's window and over a Docs tab."""
    machine = desktop.machine
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        for app, title, layer, shot in GOOGLE_WINDOWS:
            named_so = f"--class {shlex.quote(app)} --title {shlex.quote(title)}"
            desktop.as_account(f"niri msg action spawn -- kitty {named_so}")
            machine.wait_until_succeeds(
                desktop.in_session(f"niri msg --json windows | grep -qF {shlex.quote(title)}"),
                timeout=desktop_session.STARTING,
            )
            window = desktop.window_titled(title)
            try:
                desktop.as_account(f"niri msg action focus-window --id {window['id']}")
                machine.send_key("meta_l-shift-slash")
                desktop.launcher_ready()
                desktop.launcher_until(partial(on_layer, layer))
                desktop.shoot(f"launcher-shortcuts-over-{shot}-at-{at}")
                desktop.close_launcher()
            finally:
                desktop.close(window)


def test_passwords_and_a_copy_are_shot_at_each_scale(
    desktop: Desktop, at_each_scale: list[str]
) -> None:
    """Shoot Passwords, an entry read in full, the margin's mark once copied, and 1Password.

    1Password is asked through the stand-in op (tests/desktop.nix) as the place opens, and
    until it answers the place leads with 1Password locked, so it's opened again once it has.
    """
    favourite = "Lighthouse Console"
    for scale in at_each_scale:
        at = at_scale(desktop, scale)
        desktop.as_account("qs ipc call launcher collection passwords")
        desktop.launcher_ready()
        desktop.launcher_until(lambda shown: favourite in shown["rows"])
        desktop.close_launcher()
        desktop.as_account("qs ipc call launcher collection passwords")
        desktop.launcher_ready()
        desktop.launcher_until(lambda shown: named(favourite, shown) and shown["drawer"]["shown"])
        desktop.shoot(f"launcher-passwords-place-at-{at}")
        # Stepping into it reads it in full, which its drawer then says.
        desktop.follow()
        desktop.launcher_until(lambda shown: "Copy the Password" in shown["rows"])
        desktop.step_out()
        desktop.launcher_until(
            lambda shown: (
                named(favourite, shown)
                and shown["drawer"]["shown"]
                and "Health" in shown["drawer"]["drawnFacts"]
            )
        )
        desktop.shoot(f"launcher-passwords-entry-at-{at}")
        # Return copies its password and the launcher goes, the margin wearing the clipboard's
        # mark till it clears.
        since = desktop.mark()
        desktop.machine.send_key("ret")
        desktop.launcher_until(lambda shown: not shown["open"])
        desktop.reported("holds margin ", lambda marks: "clipboard" in marks.split(), since)
        desktop.shoot(f"margin-password-copied-at-{at}")
        # 1Password's program, let in, holds the place's Check Every Password after its vaults,
        # and Needing Attention only once a check has found something, which none has here.
        desktop.answer("1password")
        desktop.launcher_until(partial(named, "1Password"))
        desktop.into_program()
        desktop.launcher_until(lambda shown: "Check Every Password" in shown["rows"])
        assert "Needing Attention" not in desktop.showing()["rows"], desktop.showing()["rows"]
        desktop.shoot(f"launcher-onepassword-at-{at}")
        desktop.close_launcher()
