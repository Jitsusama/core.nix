"""A stand-in for the collector: listens where it does and keeps each line it's sent.

The collector takes one OTLP/JSON request a line on $XDG_RUNTIME_DIR/collector/lines.sock and
answers nothing. This keeps each line whole in heard.jsonl beside the socket, so a test can read
what the launcher sent.
"""

import os
import pathlib
import socket

folder = pathlib.Path(os.environ["XDG_RUNTIME_DIR"]) / "collector"
folder.mkdir(exist_ok=True)
path = folder / "lines.sock"
path.unlink(missing_ok=True)
heard = folder / "heard.jsonl"
listening = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
listening.bind(str(path))
listening.listen()
while True:
    connection, _ = listening.accept()
    with connection, connection.makefile("rb") as lines, heard.open("ab") as kept:
        for line in lines:
            kept.write(line)
            kept.flush()
