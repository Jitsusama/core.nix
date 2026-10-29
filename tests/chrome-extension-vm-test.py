"""Chromium installs the open-tabs extension from the file the Chrome module names, unasked.

tests/chrome-extension.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. The extension's id is the `values`
fixture's.
"""

from __future__ import annotations

import json
from datetime import timedelta
from typing import TYPE_CHECKING, Any

from vm_helpers import as_account

if TYPE_CHECKING:
    from test_driver.machine import QemuMachine

PROFILE = "/home/joel/profile"
LOG = "/home/joel/chromium.log"


def seen(machine: QemuMachine, extension: str) -> str:
    """Say what Chromium left of the extension: its own log, what it unpacked, what it noted."""
    _, log = machine.execute(f"tail -n 30 {LOG}")
    _, unpacked = machine.execute(f"ls -R {PROFILE}/Default/Extensions")
    _, noted = machine.execute(f"grep -l {extension} {PROFILE}/Default/*Preferences")
    return f"its log:\n{log}\nunpacked:\n{unpacked}\nnoted in: {noted}"


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
            f"test -f {PROFILE}/Default/Extensions/{extension}/1_0/manifest.json",
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
