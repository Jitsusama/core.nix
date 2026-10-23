"""A machine's clock sets from its own time server past the network's, which never answers.

tests/clock.nix runs this, and its opening comment says why timesyncd waits to be started.
The silent server the network names is a Nix value, which reaches the script in a JSON
file whose path clock.nix writes in for the marker read below, so networkd's and timesyncd's
answers are checked against the same address the machine was given.
"""

import json
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, server, start_all

values: dict[str, str] = json.loads(Path("@values@").read_text())
silent = values["silent"]

start_all()
server.wait_for_unit("chronyd.service")
machine.wait_until_succeeds(f"networkctl status eth1 | grep -q 'NTP: {silent}'", timeout=60)
machine.succeed("systemctl start systemd-timesyncd.service")
machine.succeed(f"timedatectl show-timesync -p LinkNTPServers --value | grep -qx {silent}")
machine.wait_until_succeeds(
    "timedatectl show -p NTPSynchronized --value | grep -qx yes", timeout=120
)
machine.succeed("timedatectl show-timesync -p ServerName --value | grep -qx server")
