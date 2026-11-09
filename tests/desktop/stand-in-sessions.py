"""Stand in for a crowd of pi's sessions, all owned by this one process, a few changing each beat.

It writes COUNT presence records to the folder named, as pi does each of its own: by rename, each
with its own session and address, and an owner that is this process on this boot since its
start, which is all mycelium's node asks of a record to call it live. Each beat it writes CHANGING
of them again, in turn, each flipping between working and listening, so the shell hears
sessions change as often as Joel's own do (2026-11-08: his openings heard Agents change 27 to
75 times in their first three seconds, and made 3,424 to 8,291 of its rows). Told `steps`, each
keeps working and moves only its step and what it has used, as 21 of 26 changes of Joel's 113
sessions did in 30 s (2026-11-09). It says when it has
written them on its standard output, and takes its records away when it's stopped, so no later
test finds them.

Usage: stand-in-sessions FOLDER COUNT CHANGING BEAT_SECONDS [steps]
"""

import json
import os
import signal
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from types import FrameType


def owner() -> dict[str, object]:
    """Say this process as pi's presence says its own: its pid, the boot, and when it started."""
    stat = Path("/proc/self/stat").read_text()
    # The fields after the command's closing parenthesis, the 22nd of all being its start.
    after = stat.rsplit(") ", 1)[1].split()
    return {
        "pid": os.getpid(),
        "bootId": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
        "startTicks": int(after[19]),
        "startedAt": int(time.time() * 1000),
    }


def record(at: int, *, busy: bool, whose: dict[str, object], step: int = 0) -> dict[str, object]:
    """Give one stand-in session's record, numbered `at`, working or listening, at its `step`."""
    address = f"LD{at:04d}"
    return {
        "step": "tool",
        "stepDetail": {"kind": "tool", "name": "bash", "detail": f"step {step}"},
        "usage": {"last": {"gen_ai.usage.input_tokens": step}},
        "session": f"01a1face-load-{at:04d}",
        "addresses": [address],
        "title": f"Stand-in session {at}",
        "busy": busy,
        "worksOn": f"Stand-in work {at}",
        "at": datetime.now(UTC).isoformat(),
        "owner": whose,
    }


def write(folder: Path, at: int, said: dict[str, object]) -> None:
    """Replace a record by rename, as pi does, so the node never reads half of one."""
    name = f"ld{at:04d}"
    hidden = folder / f".{name}"
    hidden.write_text(json.dumps(said))
    hidden.rename(folder / f"{name}.json")


def main() -> None:
    """Write every record, then change a few each beat until stopped, and take them away."""
    folder, count, changing, beat = (
        Path(sys.argv[1]).expanduser(),
        int(sys.argv[2]),
        int(sys.argv[3]),
        float(sys.argv[4]),
    )
    folder.mkdir(parents=True, exist_ok=True)
    whose = owner()
    steps = sys.argv[5:6] == ["steps"]
    busy = [at % 2 == 0 for at in range(count)]
    for at in range(count):
        write(folder, at, record(at, busy=busy[at], whose=whose))
    going = True

    def stop(_signal: int, _frame: FrameType | None) -> None:
        nonlocal going
        going = False

    signal.signal(signal.SIGTERM, stop)
    print(f"sessions {count} written", flush=True)
    turn = 0
    try:
        while going:
            time.sleep(beat)
            for _ in range(changing):
                at = turn % count
                busy[at] = busy[at] if steps else not busy[at]
                write(folder, at, record(at, busy=busy[at], whose=whose, step=turn))
                turn += 1
    finally:
        for at in range(count):
            (folder / f"ld{at:04d}.json").unlink(missing_ok=True)


if __name__ == "__main__":
    main()
