"""A stand-in for a kitty drawing many session windows, answering its remote control's ls.

usage: swap-in-kitty.py FOLDER NAME HEAPS SIZE [AS]

It starts HEAPS stand-in heaps of SIZE MiB as its own children, each in a scope of its own named
kitty-PID-N.scope as kitty names them, with no app id, writing each heap's pid to
FOLDER/NAME-N.pid and its own to FOLDER/NAME.pid. It then answers on the abstract socket
@kitty-PID, or @kitty-AS when AS is given, as a socket planted under another kitty's name would,
or on none when AS is none, with kitty's framing around FOLDER/NAME-shown.json, the OS windows
as kitty's ls lists them. Each
command it is sent goes on a line of FOLDER/NAME-asked.txt.
"""

import json
import os
import signal
import socket
import subprocess
import sys
from pathlib import Path

OPENS, ENDS = b"\x1bP@kitty-cmd", b"\x1b\\"

given, name, count, size, *listening = sys.argv[1:]
folder, heaps = Path(given), int(count)
listens_as = listening[0] if listening else str(os.getpid())
for number in range(1, heaps + 1):
    # S603: every word of the command is this stand-in's own or the test's.
    subprocess.Popen(  # noqa: S603
        [
            "/run/current-system/sw/bin/systemd-run",
            "--user",
            "--scope",
            f"--unit=kitty-{os.getpid()}-{number}",
            "-p",
            "MemoryZSwapMax=0",
            sys.executable,
            "/etc/swap-in/heap.py",
            size,
            str(folder / f"{name}-{number}.pid"),
        ]
    )
(folder / f"{name}.pid").write_text(str(os.getpid()))
if listens_as == "none":
    signal.pause()
server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
server.bind(f"\0kitty-{listens_as}")
server.listen()
(folder / f"{name}.listening").write_text(listens_as)
while True:
    connection, _ = server.accept()
    with connection:
        asked = b""
        while not asked.endswith(ENDS):
            chunk = connection.recv(4096)
            if not chunk:
                break
            asked += chunk
        command = json.loads(asked.removeprefix(OPENS).removesuffix(ENDS))
        with (folder / f"{name}-asked.txt").open("a") as log:
            log.write(command.get("cmd", "") + "\n")
        shown = (folder / f"{name}-shown.json").read_text()
        connection.sendall(OPENS + json.dumps({"ok": True, "data": shown}).encode() + ENDS)
