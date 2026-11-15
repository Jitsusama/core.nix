"""A switch starts again what it stopped while home-manager's setup of an account still hangs.

switch-outruns-home-manager.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py). The machine's second system changes a stand-in service, which the switch
stops and must start again, and gives alice a user unit whose start never ends, so home-manager's
setup of her account hangs. The switch runs in a unit of its own, as update-to-main runs one. The
stand-in must run again long before home-manager's setup would give up, while the switch still
waits on that setup. The tests need no value from Nix.
"""

from __future__ import annotations

import datetime as dt
from typing import TYPE_CHECKING

from test_driver.errors import RequestedAssertionFailed

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# How long the stand-in may take to run again. Home-manager waits two minutes on a unit that never
# starts, and its unit five, so a switch that starts it only after the setup takes far longer.
RUNNING_WITHIN = dt.timedelta(seconds=45)


def test_a_hung_home_manager_leaves_nothing_the_switch_stopped_down(machine: BaseMachine) -> None:
    """The stand-in runs again while the switch still waits on alice's setup."""
    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("home-manager-alice.service")
    machine.wait_for_unit("stand-in.service")
    first = machine.succeed("systemctl show stand-in -p MainPID --value").strip()
    hangs = machine.succeed("readlink -f /run/current-system/specialisation/hangs").strip()
    machine.succeed(
        f"systemd-run --unit=the-switch --property=Type=oneshot --no-block "
        f"{hangs}/bin/switch-to-configuration test"
    )
    try:
        machine.wait_until_succeeds(
            f'systemctl is-active stand-in.service && test "$(systemctl show stand-in'
            f' -p MainPID --value)" != {first}',
            timeout=RUNNING_WITHIN,
        )
    except RequestedAssertionFailed:
        # What the switch still waits on says why the stand-in isn't back.
        _, waiting = machine.execute(
            "systemctl list-jobs --no-pager; systemctl status the-switch --no-pager -l"
        )
        raise AssertionError(waiting) from None
    # The switch goes on waiting on her setup, which waits on the unit that never starts.
    state = machine.succeed("systemctl show home-manager-alice -p ActiveState --value").strip()
    assert state == "activating", f"alice's setup isn't still going: {state}"
