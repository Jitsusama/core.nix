"""Chromium installs the open-tabs extension from the file the Chrome module names, unasked.

tests/chrome-extension.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. The extension's id is the `values`
fixture's.
"""

from __future__ import annotations

import json
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any

from vm_helpers import as_account

if TYPE_CHECKING:
    from test_driver.machine import QemuMachine

PROFILE = "/home/joel/profile"
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


def tabs(machine: QemuMachine) -> list[list[str]]:
    """Give each tab chrome-open-tabs lists, as its fields."""
    listed = as_account(machine, "chrome-open-tabs list")
    return [line.split("\t") for line in listed.splitlines()]


def titled(listed: list[list[str]], title: str) -> list[str]:
    """Give the tab listed with this title."""
    return next(tab for tab in listed if tab[5] == title)


def test_the_launcher_hears_the_open_tabs_and_goes_to_or_closes_one(
    machine: QemuMachine, values: dict[str, Any]
) -> None:
    """chrome-open-tabs lists Chromium's two pages, brings one to the front, closes the other."""
    minute = int(timedelta(minutes=1).total_seconds())
    machine.succeed("pkill -u joel -f chromium; true")
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
    listing = "XDG_RUNTIME_DIR=/run/user/1000 chrome-open-tabs list"
    try:
        machine.wait_until_succeeds(
            f"[ $(su - joel -c '{listing}' | grep -c .) = 2 ]", timeout=2 * minute
        )
    except AssertionError as late:
        message = f"the host never listed both pages; {seen(machine, values['id'])}"
        raise AssertionError(message) from late
    listed = tabs(machine)
    one, two = titled(listed, "One"), titled(listed, "Two")
    assert one[1] == two[1], f"the two pages aren't in one window: {listed}"

    as_account(machine, f"chrome-open-tabs go {one[0]}")
    fronted = tabs(machine)
    assert titled(fronted, "One")[2] == "true", f"going to One left it behind: {fronted}"
    assert titled(fronted, "Two")[2] == "false", f"Two stayed in front: {fronted}"

    as_account(machine, f"chrome-open-tabs close {two[0]}")
    left = [tab[5] for tab in tabs(machine)]
    assert left == ["One"], f"closing Two left {left}"
