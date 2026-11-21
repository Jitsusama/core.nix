"""Seventy pi processes following one folder through the mesh reader, as the VM test runs them.

Each client asks the reader for the folder's .json files and waits for its listing, then the
writer publishes the one file again and again, as a pi process publishes its presence: a hidden
file renamed in. After each publish every client must be told the new text, and the slowest is
timed from the rename. It prints the slowest in milliseconds, and exits 1 when a client is told
anything else.

Usage: mesh-reader-clients.py SOCKET FOLDER CLIENTS CHANGES
"""

from __future__ import annotations

import json
import socket
import sys
import time
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Iterator

# How long a client may take to be told a change before the run fails outright.
PATIENCE_S = 10
# The pause between publishes: apart, so each is a change of its own to the reader.
APART_S = 0.2


def lines(client: socket.socket) -> Iterator[object]:
    """Give each line a client is told, parsed, as it comes."""
    buffered = b""
    while True:
        newline = buffered.find(b"\n")
        while newline < 0:
            chunk = client.recv(65536)
            if not chunk:
                return
            buffered += chunk
            newline = buffered.find(b"\n")
        line, buffered = buffered[:newline], buffered[newline + 1 :]
        yield json.loads(line)


def publish(folder: str, name: str, text: str) -> None:
    """Write a file as pi does: a hidden file, renamed in whole."""
    staged = Path(folder) / f".{name}.tmp"
    staged.write_text(text, encoding="utf-8")
    staged.rename(Path(folder) / name)


def main() -> int:
    """Follow the folder with every client, publish each change, and time the slowest."""
    path, folder, clients, changes = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
    publish(folder, "a.json", "0")
    followers = []
    for _ in range(clients):
        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.settimeout(PATIENCE_S)
        client.connect(path)
        client.sendall(json.dumps({"folder": folder, "suffix": ".json"}).encode() + b"\n")
        heard = lines(client)
        listing = []
        for line in heard:
            if line == {"ready": True}:
                break
            listing.append(line)
        if listing != [{"put": "a.json", "text": "0"}]:
            print(f"a client's listing was {listing}", file=sys.stderr)
            return 1
        followers.append(heard)
    slowest = 0.0
    for change in range(1, changes + 1):
        time.sleep(APART_S)
        published = time.monotonic()
        publish(folder, "a.json", str(change))
        for heard in followers:
            line = next(heard)
            if line != {"put": "a.json", "text": str(change)}:
                print(f"a client was told {line} after change {change}", file=sys.stderr)
                return 1
        slowest = max(slowest, time.monotonic() - published)
    print(round(slowest * 1000))
    return 0


if __name__ == "__main__":
    sys.exit(main())
