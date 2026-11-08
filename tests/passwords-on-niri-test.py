"""The Passwords place, asking the passwords daemon over its socket, on niri.

modules/quickshell/checks.nix runs this through pytest inside tests/nested-niri.sh. The daemon
is the store's own, which the check names, run as its unit runs it, on the daemons' socket, with
one password manager: a stand-in for 1Password's, named as 1Password's is
(tests/passwords-stand-in.sh), which never runs op. It starts signed out. Nothing asks it
anything until Joel steps into Passwords, which asks it once, and while it's signed out the
place leads with its row saying so, whose Return lets the launcher in. Once in, GitHub leads
as a favourite, its Return copies its password through the daemon to the clipboard, the foot
saying when it clears, resting on it reads it in full, its drawer saying its password is hidden
and never what it is, and stepped into, it holds its acts in the design's order.
"""

from __future__ import annotations

import json
import os
import subprocess
import time
from pathlib import Path
from typing import TYPE_CHECKING, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Json, Shell

SOCKET = Path(os.environ["XDG_RUNTIME_DIR"]) / "daemons" / "passwords.sock"
LOGS = Path(os.environ["LOGS"])
# What the stand-in manager was asked, a line each.
ASKED = LOGS / "passwords-asked.log"
# The stand-in's only secret, which the clipboard holds and no screen shows.
STAND_IN = "stand-in-password"


def asked() -> list[Json]:
    """Give each ask the stand-in manager has heard."""
    if not ASKED.exists():
        return []
    return [cast("Json", json.loads(line)) for line in ASKED.read_text().splitlines()]


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell, started once a run after the passwords daemon and its manager."""
    LOGS.mkdir(parents=True, exist_ok=True)
    managers = LOGS / "password-managers.json"
    stand_in = ["bash", os.environ["PASSWORDS_STAND_IN"], str(ASKED)]
    managers.write_text(json.dumps({"managers": [{"name": "1password", "command": stand_in}]}))
    environment = {**os.environ, "PASSWORDS_COPIER": "wl-copy"}
    with (LOGS / "passwords.log").open("w") as log:
        daemon = subprocess.Popen(  # noqa: S603
            [os.environ["PASSWORDS"], "daemon", str(SOCKET), str(managers)],
            stdout=log,
            stderr=log,
            env=environment,
        )
    deadline = time.monotonic() + 30
    while not SOCKET.exists():
        assert daemon.poll() is None, (LOGS / "passwords.log").read_text()
        assert time.monotonic() < deadline, "the passwords daemon never listened"
        time.sleep(0.1)
    started = launcher_on_niri.started()
    yield started
    started.stop()
    daemon.terminate()
    daemon.wait()


@pytest.fixture(autouse=True)
def launcher_closed_after(shell: Shell) -> Iterator[None]:
    """Close the launcher after each test, so one failing with it open fails alone."""
    yield
    shell.close_launcher()


def opened(shell: Shell) -> None:
    """Open Passwords, as its row at home or "passwords" typed steps into it."""
    shell.as_account(shell.ipc("collection", "passwords"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["held"] == ["passwords"])


def asks() -> list[str]:
    """Give what each ask the stand-in manager has heard asked."""
    return [str(one["ask"]) for one in asked()]


def let_in(shell: Shell) -> None:
    """Let the launcher in from the row leading Passwords, unless it's let in already."""
    if "let-in" in asks():
        return
    opened(shell)
    shell.launcher_until(lambda shown: shown["rows"][:1] == ["1Password Is Locked"])
    selected = cast("Json", shell.showing()["selected"])
    assert selected["verb"] == "unlock", selected
    # Letting in changes how it stands, so Return from the find lands on it,
    # and a second does it, as Let the Launcher In's does in Programs.
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: (
            shown["mode"] == "normal"
            and (shown["selected"] or {}).get("name") == "1Password Is Locked"
            and not shown["landedFresh"]
        )
    )
    shell.send_key("ret")
    shell.launcher_until(lambda shown: not shown["open"])
    # Let in, the daemon reads the list afresh.
    shell.launcher_until(lambda _: asks()[-2:] == ["let-in", "list"])


def test_passwords_asks_nothing_until_stepped_into_and_leads_with_letting_it_in(
    shell: Shell,
) -> None:
    """Passwords asks its manager nothing until stepped into, and leads with letting it in."""
    # The shell has watched the daemon since it started, and asked nothing of
    # the manager, so 1Password would have asked Joel nothing.
    shell.launcher_until(lambda shown: not shown["open"])
    assert asked() == [], asked()
    opened(shell)
    # Stepping in is Joel's key, so the list is asked once, as his.
    shell.launcher_until(lambda _: asks() == ["list"])
    assert asked()[0]["asked"] is True, asked()
    # Refused, as 1Password is when its prompt is dismissed, so the place
    # leads with the row that lets the launcher in.
    shell.launcher_until(lambda shown: shown["rows"][:1] == ["1Password Is Locked"])
    shell.close_launcher()
    let_in(shell)


def test_an_entrys_return_copies_its_password_and_the_foot_says_when_it_clears(
    shell: Shell,
) -> None:
    """An entry's Return copies its password and stays, the foot saying when it clears."""
    let_in(shell)
    opened(shell)
    # Let in, the favourite leads, then what changed lately.
    shell.launcher_until(lambda shown: shown["rows"][:2] == ["GitHub", "Bank Card"])
    row = cast("Json", shell.showing()["selected"])
    assert row["name"] == "GitHub", row
    assert row["note"] == "ada", row
    assert row["verb"] == "copy", row
    # Rested on, it's read in full first, so the copy has nothing more to read.
    shell.launcher_until(
        lambda shown: "One-Time Code" in str((shown["selected"] or {}).get("drawer") or {})
    )
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: (
            shown["open"]
            and str(shown["footer"]).startswith("copied GitHub's password \u00b7 clears in ")
        )
    )
    assert shell.copied() == STAND_IN
    # Nothing on screen says it: not the row, nor its drawer, nor the foot.
    assert STAND_IN not in json.dumps(shell.showing())


def test_resting_on_an_entry_reads_it_in_full_and_it_holds_its_acts_in_order(
    shell: Shell,
) -> None:
    """Resting on an entry reads it in full, and stepped into, it holds its acts in order."""
    let_in(shell)
    opened(shell)
    shell.launcher_until(lambda shown: shown["rows"][:1] == ["GitHub"])
    # Rested on, it's read in full, not as Joel's own ask, since the manager has let the
    # launcher in, so 1Password would ask him nothing; its drawer says what that told.
    shell.launcher_until(lambda _: "get" in asks())
    gets = [one for one in asked() if one["ask"] == "get"]
    assert all(one.get("asked") is not True for one in gets), gets
    shell.launcher_until(
        lambda shown: "One-Time Code" in str((shown["selected"] or {}).get("drawer") or {})
    )
    shown = shell.showing()
    drawer = cast("Json", (shown["selected"] or {}).get("drawer") or {})
    facts = {fact.get("label"): fact.get("value") for fact in drawer.get("facts", [])}
    assert facts.get("Password") == "hidden", drawer
    assert facts.get("One-Time Code") == "set up", drawer
    assert facts.get("Strength") == "fantastic", drawer
    assert STAND_IN not in json.dumps(shown)
    # Read already, stepping in holds every act in the design's order, its one-time code's
    # among them.
    shell.step_in("ctrl-bracketright")
    acts = [
        "Copy the Password",
        "Copy the One-Time Code",
        "Copy the Username",
        "Open the Website",
        "Show in 1Password",
        "Copy the Link",
    ]
    shell.launcher_until(lambda shown: shown["rows"] == acts)
    shell.launcher_until(lambda shown: shown["held"] == ["passwords", "GitHub"])
