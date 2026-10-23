"""A reboot whose shutdown would hang goes on anyway, and says what it told Plymouth.

tests/shutdown.nix runs this, and its opening comment says what the test proves. The machine
exits on any reset, so each reboot ends in wait_for_shutdown and the next subtest starts the
machine again. The programs the stuck service and the leftover run are named by their paths in
the store, from the JSON file tests/shutdown.nix swaps in for the marker below.
"""

import json
import re
import time
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest, t

# coreutils' sleep and bash's sh, by their paths in the store.
values = json.loads(Path("@values@").read_text())
sleep: str = values["sleep"]
sh: str = values["sh"]


def reboot() -> tuple[float, int]:
    """Reboot the machine once it's up, giving when, and how much the console had said."""
    machine.wait_for_unit("multi-user.target")
    said = len(machine.get_console_log())
    machine.succeed("systemctl --no-block reboot")
    return time.monotonic(), said


def told(said: int) -> list[str]:
    """Give what the machine told Plymouth since the console had said that much.

    It reads the kernel's own line, stamped with its time, and not the journal's copy.
    """
    return re.findall(r"\] told plymouth --status=(\w+)", machine.get_console_log()[said:])


with subtest("a healthy reboot tells Plymouth nothing"):
    started, said = reboot()
    machine.wait_for_shutdown(timeout=timedelta(seconds=30))
    t.assertEqual(told(said), [])

with subtest("a service that won't stop holds a reboot a minute"):
    machine.start()
    machine.wait_for_unit("multi-user.target")
    machine.succeed(
        "systemd-run --unit=stuck -p Type=exec -p TimeoutStopSec=infinity"
        f" -p ExecStop='{sleep} infinity'"
        f" {sleep} infinity"
    )
    started, said = reboot()
    machine.wait_for_shutdown(timeout=timedelta(seconds=120))
    took = time.monotonic() - started
    t.assertGreater(took, 55, "the reboot didn't wait for the service")
    t.assertLess(took, 90, "the reboot waited past the minute")
    t.assertEqual(told(said), ["forced"])

with subtest("a leftover that ignores SIGTERM can't hold the last stage"):
    machine.start()
    machine.wait_for_unit("multi-user.target")
    machine.succeed("test -e /dev/watchdog0")
    machine.succeed(
        f"systemd-run --unit=leftover -p KillMode=none {sh} -c"
        f" 'trap \"\" TERM; exec {sleep} infinity'"
    )
    started, said = reboot()
    machine.wait_for_qmp_event(
        lambda event: event["event"] == "WATCHDOG", timeout=timedelta(seconds=60)
    )
    took = time.monotonic() - started
    machine.wait_for_shutdown(timeout=timedelta(seconds=30))
    t.assertGreater(took, 25, "the watchdog fired before its 30 seconds")
    t.assertEqual(told(said), ["leftovers"])
