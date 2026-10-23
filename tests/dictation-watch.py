"""Stand in for dictation's service, to hold the shell's watch to it.

It runs the shell (tests/dictation.qml), whose path comes first, and answers
on the socket as the service does. The shell asks to watch and follows each
state said. A service that stops is watched again once it's started from its
socket. When nothing listens, the shell tries once and then waits, rather
than looping. It exits 0 once every case holds and 1 saying which didn't.
"""

import os
import queue
import socket
import subprocess
import sys
import threading
from pathlib import Path
from typing import NoReturn

WAIT = 30
runtime = Path(os.environ["XDG_RUNTIME_DIR"])
path = runtime / "dictation.sock"
said: queue.Queue[str] = queue.Queue()


def fail(message: str) -> NoReturn:
    """Say which case didn't hold, and end the shell and the test."""
    print(f"FAIL {message}", file=sys.stderr)
    shell.kill()
    sys.exit(1)


def listening(at: Path = path) -> socket.socket:
    """Listen on a socket, as systemd does for the service."""
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(str(at))
    server.listen()
    server.settimeout(WAIT)
    return server


def watched(server: socket.socket, case: str) -> socket.socket:
    """Wait for the shell to connect and ask to watch."""
    try:
        client, _ = server.accept()
    except TimeoutError:
        fail(f"{case}: the shell never connected")
    client.settimeout(WAIT)
    if client.recv(64) != b"watch\n":
        fail(f"{case}: the shell didn't ask to watch")
    return client


def heard(state: str, case: str) -> None:
    """Wait for the shell to say it came to a state."""
    while True:
        try:
            line = said.get(timeout=WAIT)
        except queue.Empty:
            fail(f"{case}: the shell never came to {state}")
        if line.strip() == f"state {state}":
            return


def read(told: socket.socket) -> None:
    """Keep each state the shell says over its report socket."""
    for line in told.makefile("r"):
        said.put(line)


reports = listening(runtime / "report.sock")
server = listening()
shell = subprocess.Popen([sys.argv[1], "-p", "shell.qml"])  # noqa: S603
# Nothing's said until the report is open, so nothing said goes unheard.
try:
    told, _ = reports.accept()
except TimeoutError:
    fail("the shell never opened its report")
told.settimeout(None)
threading.Thread(target=read, args=(told,), daemon=True).start()

client = watched(server, "the start")
client.sendall(b"idle\n")
for state in ["listening", "transcribing", "idle"]:
    client.sendall(f"{state}\n".encode())
    heard(state, f"saying {state}")

# The service stops while listening, and its socket stays: the shell goes
# back to idle and watches again, which starts it.
client.sendall(b"listening\n")
heard("listening", "listening again")
client.close()
heard("idle", "the service stopping")
client = watched(server, "the service started again")
client.sendall(b"listening\n")
heard("listening", "watching again")

# Now nothing listens at all: the shell goes back to idle, tries once, and
# doesn't come back on its own.
client.close()
server.close()
path.unlink()
heard("idle", "the socket going")
server = listening()
server.settimeout(3)
try:
    server.accept()
    fail("a socket that wouldn't connect: the shell kept trying")
except TimeoutError:
    pass

shell.kill()
print("every case holds", file=sys.stderr)
