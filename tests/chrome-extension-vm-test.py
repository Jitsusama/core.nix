"""Chromium says its open tabs: through the open-tabs extension, and in its session file.

tests/chrome-extension.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. The extension's id is the `values`
fixture's.
"""

from __future__ import annotations

import base64
import json
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any

from vm_helpers import as_account

if TYPE_CHECKING:
    from test_driver.machine import QemuMachine

PROFILE = "/home/joel/profile"
SESSIONED = "/home/joel/sessioned"
LOG = "/home/joel/chromium.log"


def seen(machine: QemuMachine, extension: str) -> str:
    """Say what Chromium left of the extension: its own log, what it unpacked, what it noted."""
    _, log = machine.execute(f"tail -n 30 {LOG} 2>&1")
    _, unpacked = machine.execute(f"ls -R {PROFILE}/Default/Extensions 2>&1")
    _, noted = machine.execute(f"grep -l {extension} {PROFILE}/Default/*Preferences 2>&1")
    _, running = machine.execute("pgrep -a -u joel 2>&1; ls -la /run/user/1000 2>&1")
    _, hosts = machine.execute("cat /etc/chromium/native-messaging-hosts/*.json 2>&1")
    return (
        f"its log:\n{log}\nunpacked:\n{unpacked}\nnoted in: {noted}\n"
        f"running, and the runtime folder:\n{running}\nits hosts:\n{hosts}"
    )


def test_chromium_installs_the_extension_named_in_its_preferences_file(
    machine: QemuMachine, values: dict[str, Any]
) -> None:
    """Chromium unpacks the extension into its profile and grants it what it asks for."""
    extension = values["id"]
    machine.wait_for_unit("multi-user.target")
    machine.succeed(f"test -f /run/current-system/sw/share/chromium/extensions/{extension}.json")
    as_account(
        machine,
        f"chromium --headless=new --no-sandbox --user-data-dir={PROFILE} about:blank "
        f"> {LOG} 2>&1 &",
        account="joel",
    )
    try:
        machine.wait_until_succeeds(
            f"ls {PROFILE}/Default/Extensions/{extension}/*/manifest.json",
            timeout=int(timedelta(minutes=3).total_seconds()),
        )
    except AssertionError as late:
        message = f"Chromium never unpacked the extension; {seen(machine, extension)}"
        raise AssertionError(message) from late
    # Chromium says what it granted the extension in one of the profile's preferences once it
    # has installed it, which it writes a few seconds after.
    try:
        machine.wait_until_succeeds(
            f"grep -l {extension} {PROFILE}/Default/*Preferences",
            timeout=int(timedelta(minutes=2).total_seconds()),
        )
    except AssertionError as late:
        message = f"Chromium never noted the extension; {seen(machine, extension)}"
        raise AssertionError(message) from late
    noting = machine.succeed(f"grep -l {extension} {PROFILE}/Default/*Preferences").split("\n")
    # Each file may hold some of its settings, so they're read together.
    its: dict[str, Any] = {}
    for preferences in filter(None, noting):
        read = json.loads(machine.succeed(f"cat '{preferences}'"))
        its |= read.get("extensions", {}).get("settings", {}).get(extension, {})
    assert "granted_permissions" in its, f"nothing granted it in {noting}: {its}"
    granted = its["granted_permissions"]["api"]
    assert sorted(granted) == ["nativeMessaging", "tabs"], granted
    assert its.get("disable_reasons", []) == [], its


def command(asked: list[str], tab: str = "") -> str:
    """Give a command of Chrome's source as the launcher runs it, given a tab's id."""
    return " ".join(shlex.quote(part.replace("{id}", tab)) for part in asked)


def tabs(machine: QemuMachine) -> list[list[str]]:
    """Give each tab the host says is open, as its fields, as the programs daemon asks it."""
    listed = as_account(machine, "chrome-open-tabs tabs")
    return [line.split("\t") for line in listed.splitlines()]


def titled(listed: list[list[str]], title: str) -> list[str]:
    """Give the tab listed with this title."""
    return next(tab for tab in listed if tab[5] == title)


def test_the_launcher_hears_the_open_tabs_and_goes_to_or_closes_one(
    machine: QemuMachine, values: dict[str, Any]
) -> None:
    """The host lists Chromium's two pages, and Chrome's source goes to one, closes the other."""
    source = values["source"]
    assert {"go", "close"} <= source.keys(), f"the source found no host to act through: {source}"
    asking = source.get("session", {}).get("host")
    assert asking is True, f"the source doesn't say to ask the host: {source}"
    minute = int(timedelta(minutes=1).total_seconds())
    # Run alone, nothing before it has waited for the machine to start, and
    # there's no Chromium to stop, which pkill fails on.
    machine.wait_for_unit("multi-user.target")
    machine.succeed("pkill -u joel -f chromium || true")
    machine.wait_until_fails("pgrep -u joel -f chromium", timeout=minute)
    titles = ("One", "Two")
    pages = " ".join(shlex.quote(f"data:text/html,<title>{title}</title>") for title in titles)
    # Without a screen Chromium opens one page alone, so it gets a virtual one.
    as_account(
        machine,
        "xvfb-run -a chromium --ozone-platform=x11 --no-sandbox --disable-gpu --no-first-run "
        f"--password-store=basic --user-data-dir={PROFILE} {pages} > {LOG} 2>&1 &",
        account="joel",
    )
    listing = "XDG_RUNTIME_DIR=/run/user/1000 chrome-open-tabs tabs"
    try:
        machine.wait_until_succeeds(
            f"[ $(su - joel -c '{listing}' | grep -c .) = 2 ]", timeout=2 * minute
        )
    except AssertionError as late:
        message = f"the host never listed both pages; {seen(machine, values['id'])}"
        raise AssertionError(message) from late
    listed = tabs(machine)
    assert all(tab[0].isdigit() for tab in listed), f"not each tab by the host's id: {listed}"
    one, two = titled(listed, "One"), titled(listed, "Two")
    assert one[1] == two[1], f"the two pages aren't in one window: {listed}"

    as_account(machine, command(source["go"], one[0]))
    fronted = tabs(machine)
    assert titled(fronted, "One")[2] == "true", f"going to One left it behind: {fronted}"
    assert titled(fronted, "Two")[2] == "false", f"Two stayed in front: {fronted}"

    as_account(machine, command(source["close"], two[0]))
    left = [tab[5] for tab in tabs(machine)]
    assert left == ["One"], f"closing Two left {left}"


def written(machine: QemuMachine) -> str:
    """Say what the session file holds, what's served, and what Chromium itself has open."""
    sessions = f"{SESSIONED}/Default/Sessions"
    _, listed = machine.execute(f"ls -la {sessions} 2>&1")
    _, read = machine.execute(f"su - joel -c 'chrome-session {SESSIONED}/Default' 2>&1")
    _, served = machine.execute("curl -sS http://pages.example.org/one.html 2>&1")
    _, live = machine.execute(
        "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 chrome-open-tabs tabs' 2>&1"
    )
    _, encoded = machine.execute(f"cat {sessions}/Session_* | base64 -w0")
    raw = base64.b64decode(encoded)
    titles = {title: title.encode("utf-16-le") in raw for title in ("One", "Two")}
    return (
        f"sessions:\n{listed}\nread:\n{read}\nserved: {served}\nChromium has open:\n{live}\n"
        f"each title in the file's bytes: {titles}, {len(raw)} bytes"
    )


def test_chrome_session_reads_every_tab_from_the_session_file(machine: QemuMachine) -> None:
    """chrome-session reads both pages Chromium opened, from the session file it writes."""
    minute = int(timedelta(minutes=1).total_seconds())
    machine.succeed("pkill -u joel -f chromium || true")
    machine.wait_until_fails("pgrep -u joel -f chromium", timeout=minute)
    machine.wait_for_open_port(80)
    pages = " ".join(f"http://pages.example.org/{name}.html" for name in ("one", "two"))
    as_account(
        machine,
        "xvfb-run -a chromium --ozone-platform=x11 --no-sandbox --disable-gpu --no-first-run "
        f"--password-store=basic --user-data-dir={SESSIONED} {pages} > {LOG} 2>&1 &",
        account="joel",
    )
    reading = f"chrome-session {SESSIONED}/Default"
    # Chromium here writes the session as it starts, before either page has loaded, so their
    # titles aren't in it yet; the case reads what that holds. Tests in session.rs read titles.
    try:
        machine.wait_until_succeeds(
            f"[ $(su - joel -c '{reading}' | grep -c pages.example.org) = 2 ]", timeout=2 * minute
        )
    except AssertionError as late:
        message = f"the session never held both pages; {written(machine)}"
        raise AssertionError(message) from late
    listed = [line.split("\t") for line in as_account(machine, reading).splitlines()]
    addresses = [f"http://pages.example.org/{name}.html" for name in ("one", "two")]
    assert sorted(tab[4] for tab in listed) == addresses, f"not the two pages: {listed}"
    one, two = sorted(listed, key=lambda tab: tab[4])
    assert one[1] == two[1], f"the two pages aren't in one window: {listed}"
    fronts = [tab for tab in listed if tab[2] == "true"]
    assert len(fronts) == 1, f"not one page in front: {listed}"
    now = int(machine.succeed("date +%s%3N"))
    looked = now - int(fronts[0][3])
    assert 0 <= looked < 10 * minute * 1000, f"the front page last looked at {looked} ms ago"
