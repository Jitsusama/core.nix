"""Write which memory modules the firmware lists, as JSON, for the launcher's Memory to read.

The firmware lists each memory module as an SMBIOS memory device (type 17), which the kernel
shows in /sys/firmware/dmi/entries, readable by root alone. nixos.nix runs this as root once at
boot with that folder and the file to write, and moves the file into place. Each module is its
locator, size, type, speed, maker and part, an empty string where the firmware says nothing,
and a machine whose firmware lists none gets an empty list. The SMBIOS specification (DSP0134,
7.18) gives each field's place and encoding.
"""

import json
import sys
from pathlib import Path

MEMORY_DEVICE = 17

# Every structure opens with its type, its formatted area's length and its handle.
HEADER = 4

# Where each field sits in a memory device's formatted area, by the specification's offsets.
SIZE = 0x0C
DEVICE_LOCATOR = 0x10
BANK_LOCATOR = 0x11
MEMORY_TYPE = 0x12
SPEED = 0x15
MANUFACTURER = 0x17
PART_NUMBER = 0x1A
EXTENDED_SIZE = 0x1C
CONFIGURED_SPEED = 0x20
EXTENDED_SPEED = 0x54
EXTENDED_CONFIGURED_SPEED = 0x58

# A size of 0 is an empty slot, and all ones an unknown size. 0x7FFF says the size is in the
# extended field, in mebibytes; otherwise the top bit says kibibytes rather than mebibytes.
EMPTY_SLOT = 0
UNKNOWN_WORD = 0xFFFF
SIZE_IS_EXTENDED = 0x7FFF
SIZE_IN_KIBIBYTES = 0x8000
SIZE_BITS = 0x7FFF
EXTENDED_SIZE_BITS = 0x7FFFFFFF
KIBIBYTES_IN_A_MEBIBYTE = 1024
KIBIBYTES_IN_A_GIBIBYTE = 1024 * 1024

# The memory types the specification names, by their codes, as dmidecode spells them. Other
# and Unknown say nothing a person can use, so they read as nothing said.
MEMORY_TYPES = {
    0x03: "DRAM",
    0x04: "EDRAM",
    0x05: "VRAM",
    0x06: "SRAM",
    0x07: "RAM",
    0x08: "ROM",
    0x09: "Flash",
    0x0A: "EEPROM",
    0x0B: "FEPROM",
    0x0C: "EPROM",
    0x0D: "CDRAM",
    0x0E: "3DRAM",
    0x0F: "SDRAM",
    0x10: "SGRAM",
    0x11: "RDRAM",
    0x12: "DDR",
    0x13: "DDR2",
    0x14: "DDR2 FB-DIMM",
    0x18: "DDR3",
    0x19: "FBD2",
    0x1A: "DDR4",
    0x1B: "LPDDR",
    0x1C: "LPDDR2",
    0x1D: "LPDDR3",
    0x1E: "LPDDR4",
    0x1F: "Logical non-volatile device",
    0x20: "HBM",
    0x21: "HBM2",
    0x22: "DDR5",
    0x23: "LPDDR5",
    0x24: "HBM3",
}

# SMBIOS has no code for LPDDR5X, so firmware lists it as LPDDR5. JEDEC's LPDDR5 runs at
# 6400 MT/s at most, so LPDDR5 rated faster than that can only be LPDDR5X.
LPDDR5 = "LPDDR5"
LPDDR5X = "LPDDR5X"
LPDDR5_FASTEST = 6400

# What firmware writes in a string it has nothing to put in, which says nothing either.
NOTHING_SAID = {"", "not specified", "unknown", "undefined", "none", "to be filled by o.e.m."}


def number(raw: bytes, offset: int, width: int) -> int | None:
    """Give the little-endian number at an offset of the formatted area, or None past its end."""
    if offset + width > raw[1]:
        return None
    return int.from_bytes(raw[offset : offset + width], "little")


def strings_of(raw: bytes) -> list[str]:
    """Give the strings that follow the formatted area, in order, each trimmed."""
    texts = []
    for piece in raw[raw[1] :].split(b"\0"):
        if not piece:
            break
        texts.append(piece.decode("ascii", errors="replace").strip())
    return texts


def string_at(raw: bytes, offset: int, texts: list[str]) -> str:
    """Give the string a field names by its number, counted from one, or "" for none."""
    index = number(raw, offset, 1)
    if not index or index > len(texts):
        return ""
    text = texts[index - 1]
    return "" if text.lower() in NOTHING_SAID else text


def kibibytes_of(raw: bytes) -> int | None:
    """Give a module's size in kibibytes, 0 for an empty slot, or None when it isn't known."""
    size = number(raw, SIZE, 2)
    if size is None or size == UNKNOWN_WORD:
        return None
    if size == SIZE_IS_EXTENDED:
        extended = number(raw, EXTENDED_SIZE, 4)
        if extended is None:
            return None
        return (extended & EXTENDED_SIZE_BITS) * KIBIBYTES_IN_A_MEBIBYTE
    if size & SIZE_IN_KIBIBYTES:
        return size & SIZE_BITS
    return size * KIBIBYTES_IN_A_MEBIBYTE


def size_said(kibibytes: int | None) -> str:
    """Say a size in the largest unit it's a whole number of, as dmidecode does."""
    if kibibytes is None:
        return ""
    if kibibytes % KIBIBYTES_IN_A_GIBIBYTE == 0:
        return f"{kibibytes // KIBIBYTES_IN_A_GIBIBYTE} GB"
    if kibibytes % KIBIBYTES_IN_A_MEBIBYTE == 0:
        return f"{kibibytes // KIBIBYTES_IN_A_MEBIBYTE} MB"
    return f"{kibibytes} kB"


def speed_at(raw: bytes, offset: int, extended_offset: int) -> int | None:
    """Give a speed in MT/s, from its extended field when the word is all ones."""
    speed = number(raw, offset, 2)
    if speed == UNKNOWN_WORD:
        speed = number(raw, extended_offset, 4)
    return speed or None


def speed_said(raw: bytes) -> str:
    """Say the speed the memory runs at, or the fastest it's rated for when that's not said."""
    speed = speed_at(raw, CONFIGURED_SPEED, EXTENDED_CONFIGURED_SPEED) or speed_at(
        raw, SPEED, EXTENDED_SPEED
    )
    return f"{speed} MT/s" if speed else ""


def type_said(raw: bytes) -> str:
    """Say the memory's type as the specification names it, LPDDR5X where it can only be that."""
    said = MEMORY_TYPES.get(number(raw, MEMORY_TYPE, 1) or 0, "")
    rated = speed_at(raw, SPEED, EXTENDED_SPEED) or speed_at(
        raw, CONFIGURED_SPEED, EXTENDED_CONFIGURED_SPEED
    )
    if said == LPDDR5 and rated is not None and rated > LPDDR5_FASTEST:
        return LPDDR5X
    return said


def module_of(raw: bytes) -> dict[str, str] | None:
    """Give a memory device as the launcher reads it, or None for an empty slot."""
    kibibytes = kibibytes_of(raw)
    if kibibytes == EMPTY_SLOT:
        return None
    texts = strings_of(raw)
    return {
        "locator": string_at(raw, DEVICE_LOCATOR, texts) or string_at(raw, BANK_LOCATOR, texts),
        "size": size_said(kibibytes),
        "type": type_said(raw),
        "speed": speed_said(raw),
        "manufacturer": string_at(raw, MANUFACTURER, texts),
        "part": string_at(raw, PART_NUMBER, texts),
    }


def instance_of(entry: Path) -> int:
    """Give the number the kernel gives an entry among those of its type, from its name."""
    return int(entry.name.rpartition("-")[2])


def modules_in(entries: Path) -> list[dict[str, str]]:
    """Give every memory module the firmware lists, in its order, leaving out empty slots."""
    if not entries.is_dir():
        return []
    modules = []
    for entry in sorted(entries.glob(f"{MEMORY_DEVICE}-*"), key=instance_of):
        raw = (entry / "raw").read_bytes()
        if len(raw) < HEADER or raw[0] != MEMORY_DEVICE or len(raw) < raw[1]:
            continue
        module = module_of(raw)
        if module is not None:
            modules.append(module)
    return modules


def main() -> None:
    """Write the modules found in the entries folder given to the file given, readable by all."""
    entries, written = (Path(argument) for argument in sys.argv[1:])
    written.write_text(json.dumps({"modules": modules_in(entries)}) + "\n")
    written.chmod(0o644)


if __name__ == "__main__":
    main()
