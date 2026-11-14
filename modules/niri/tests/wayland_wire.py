"""Frames Wayland requests and reads their arguments, for niri's checks' stand-in clients.

The stand-ins speak the wire protocol themselves rather than through a client library, so each
check sees exactly what it sends and when; this is the framing they share.
"""

import struct


def message(target: int, opcode: int, payload: bytes = b"") -> bytes:
    """Frame a request to the object `target`, as the wire protocol does."""
    return struct.pack("<II", target, (8 + len(payload)) << 16 | opcode) + payload


def string(text: str) -> bytes:
    """Encode a string argument: its length with its NUL, then it, padded to 4 bytes."""
    data = text.encode() + b"\0"
    return struct.pack("<I", len(data)) + data + b"\0" * (-len(data) % 4)


def read_string(payload: bytes, at: int) -> tuple[str, int]:
    """Read the string argument at `at` in a payload, and where the next one starts."""
    (length,) = struct.unpack_from("<I", payload, at)
    text = payload[at + 4 : at + 4 + length - 1].decode()
    return text, at + 4 + length + (-length % 4)
