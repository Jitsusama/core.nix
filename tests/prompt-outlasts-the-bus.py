"""Whether a D-Bus call from the account's shell outlasts systemd's own 25 seconds.

run0 asks for polkit's prompt inside one D-Bus call, so a call that gives up cancels a prompt
nobody has reached yet. This answers a call made with busctl, as run0's own library makes it,
only after 26 seconds, with SYSTEMD_BUS_TIMEOUT as the account's shells set it. The
a-prompt-waits-for-a-person check runs it on a session bus of its own, as
`prompt-outlasts-the-bus.py <timeout> <busctl>`.
"""

import os
import subprocess
import sys
import time

from jeepney import message_bus, new_method_return
from jeepney.io.blocking import open_dbus_connection
from jeepney.low_level import HeaderFields

NAME = "test.Prompt"
# Past systemd's 25 seconds, and as long as a person a room away might take.
ANSWERED_AFTER = 26


def main() -> int:
    """Answer one busctl call late, and say whether it waited for the answer."""
    timeout, busctl = sys.argv[1], sys.argv[2]
    connection = open_dbus_connection(bus="SESSION")
    connection.send_and_get_reply(message_bus.RequestName(NAME))
    # busctl's path is the check's own, from Nix.
    asker = subprocess.Popen(  # noqa: S603
        [busctl, "--user", "call", NAME, "/", NAME, "Ask"],
        env={**os.environ, "SYSTEMD_BUS_TIMEOUT": timeout},
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    asked = connection.receive(timeout=10)
    # The bus says the name is ours first.
    while asked.header.fields.get(HeaderFields.member) != "Ask":
        asked = connection.receive(timeout=10)
    time.sleep(ANSWERED_AFTER)
    connection.send(new_method_return(asked, "s", ("answered",)))
    said, _ = asker.communicate(timeout=30)
    if asker.returncode != 0 or "answered" not in said:
        print(f"a call with SYSTEMD_BUS_TIMEOUT={timeout!r} gave up first: {said.strip()}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
