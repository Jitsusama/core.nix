"""A stand-in for niri's socket that answers FocusedWindow from a file.

usage: swap-in-niri.py SOCKET FOCUS

Each connection's request is read and answered with niri's shape,
{"Ok": {"FocusedWindow": WINDOW}}, WINDOW being FOCUS's JSON, or null
while FOCUS is missing. The swap-in VM test writes FOCUS to move the focus.
"""

import json
import socket
import sys
from pathlib import Path

path, focus = Path(sys.argv[1]), Path(sys.argv[2])
path.unlink(missing_ok=True)
server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
server.bind(str(path))
server.listen()
while True:
    connection, _ = server.accept()
    with connection, connection.makefile("rw") as stream:
        stream.readline()
        try:
            window = json.loads(focus.read_text())
        except FileNotFoundError:
            window = None
        stream.write(json.dumps({"Ok": {"FocusedWindow": window}}) + "\n")
