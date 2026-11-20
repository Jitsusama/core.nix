"""Copies a secret through Passwords' copier on niri, and checks it never leaves the copier.

Stock wl-copy fed from its standard input, as the daemon feeds it, has a forked cat write what
it's given to $TMPDIR/wl-copy-buffer-XXXXXX/stdin, and serves every paste by running "cat",
found on PATH, on that file. Any process of the account's sees and writes /tmp, fenced or not,
so one watching it could open the file before wl-copy unlinks it and read the password, around
niri's hold on a sensitive offer; a cat planted on PATH would be handed it at each paste.
Passwords' copier keeps a sensitive copy in its own memory instead
(wl-copy-keeps-a-secret-in-memory.patch). This runs it as the daemon does, with an empty TMPDIR
watched for anything made in it and a cat and an xdg-mime first on PATH that note being run,
the cat then running the real one so a copier that leans on it still holds its copy, and checks:

1. The secret pastes on Joel's own display, whole.
2. Nothing at all is made under TMPDIR while it's copied and pasted.
3. Nothing on PATH runs, neither cat nor xdg-mime.
4. The copier can't be read through /proc by the account's other processes.
5. The copier's memory holding the secret is locked, so it never reaches swap.
6. A standard input it can't read is refused, making nothing under TMPDIR and running nothing.
7. A standard input past the copier's cap is refused, rather than held in part.

Every case is checked and reported before it fails.

usage: copier-keeps-a-secret-in-memory.py <copier> <shell for the stand-ins>
"""

import contextlib
import ctypes
import os
import secrets
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

# The type the daemon copies as.
TEXT = "text/plain;charset=utf-8"
# How long the copier is given to take the clipboard, in seconds.
PATIENCE = 20.0
# inotify's flags, from <sys/inotify.h>: what's made or moved into a folder, and a descriptor
# whose reads don't wait.
IN_CREATE = 0x100
IN_MOVED_TO = 0x80
IN_NONBLOCK = 0o4000
# A standard input past the copier's cap, of a MiB, in bytes.
TOO_LONG = 2 * 1024 * 1024

LIBC = ctypes.CDLL(None, use_errno=True)


class Cases:
    """The seven cases, run against one copier, with what failed in each."""

    def __init__(self, copier: str, shell: str, folder: Path) -> None:
        """Run them against `copier`, its stand-ins run by `shell` and its TMPDIR in `folder`."""
        self.copier = copier
        self.failures: list[str] = []
        self.made = folder / "tmp"
        self.made.mkdir()
        self.ran = folder / "ran"
        self.ran.touch()
        planted = folder / "planted"
        planted.mkdir()
        then = {"cat": f'exec {shutil.which("cat")} "$@"', "xdg-mime": "exit 1"}
        for name, ending in then.items():
            stand_in = planted / name
            stand_in.write_text(f"#!{shell}\necho {name} >> {self.ran}\n{ending}\n")
            stand_in.chmod(0o755)
        self.environment = {
            **os.environ,
            "PATH": f"{planted}:{os.environ['PATH']}",
            "TMPDIR": str(self.made),
        }
        self.watch = LIBC.inotify_init1(IN_NONBLOCK)
        if (
            self.watch < 0
            or LIBC.inotify_add_watch(self.watch, bytes(self.made), IN_CREATE | IN_MOVED_TO) < 0
        ):
            raise OSError(ctypes.get_errno(), "couldn't watch the copier's TMPDIR")

    def check(self, case: str, failed: str | None) -> None:
        """Keep a case's failure, or say it held."""
        if failed is None:
            print(f"{case}: held")
        else:
            self.failures.append(f"{case}: {failed}")

    def copier_started(self, stdin: int) -> subprocess.Popen[bytes]:
        """Start the copier as the daemon does, reading from `stdin`."""
        return subprocess.Popen(
            [self.copier, "--sensitive", "--foreground", "--type", TEXT],
            stdin=stdin,
            stdout=subprocess.DEVNULL,
            env=self.environment,
        )

    def anything_made(self) -> str | None:
        """Say what was made under TMPDIR since it was last asked, if anything was."""
        try:
            heard = os.read(self.watch, 65536)
        except BlockingIOError:
            heard = b""
        named = sorted(str(path.relative_to(self.made)) for path in self.made.rglob("*"))
        for made in self.made.iterdir():
            if made.is_dir():
                shutil.rmtree(made)
            else:
                made.unlink()
        if heard or named:
            return f"TMPDIR was written: {len(heard)} bytes of events, holding {named}"
        return None

    def anything_ran(self) -> str | None:
        """Say what on PATH ran since it was last asked, if anything did."""
        ran = self.ran.read_text().split()
        self.ran.write_text("")
        return f"ran {ran} from PATH" if ran else None

    def copied(self) -> None:
        """Run cases 1 to 5: a secret copied and pasted, and the copier holding it."""
        held = f"secret-{secrets.token_hex(8)}"
        reading, writing = os.pipe()
        copier = self.copier_started(reading)
        os.close(reading)
        os.write(writing, held.encode())
        os.close(writing)
        try:
            pasted = pasted_once_held(held)
            self.check("the secret pastes whole", None if pasted else "it never pasted")
            self.check("nothing is made under TMPDIR", self.anything_made())
            self.check("nothing on PATH runs", self.anything_ran())
            self.check("it can't be read through /proc", ended(copier) or readable(copier.pid))
            self.check("its secret is locked in memory", ended(copier) or unlocked(copier.pid))
        finally:
            copier.kill()
            copier.wait()

    def refused(self) -> None:
        """Run cases 6 and 7: a standard input it can't read, then one past its cap."""
        unreadable = os.open(self.made.parent, os.O_RDONLY | os.O_DIRECTORY)
        copier = self.copier_started(unreadable)
        os.close(unreadable)
        failed = refusal(ended_within(copier))
        self.check("an unreadable stdin is refused", failed or self.anything_made())
        self.check("an unreadable stdin runs nothing", self.anything_ran())

        reading, writing = os.pipe()
        copier = self.copier_started(reading)
        os.close(reading)
        with contextlib.suppress(BrokenPipeError):
            os.write(writing, b"x" * TOO_LONG)
        os.close(writing)
        failed = refusal(ended_within(copier))
        self.check("a stdin past the cap is refused", failed or self.anything_made())


def pasted_once_held(held: str) -> bool:
    """Answer whether Joel's display pastes the secret, waiting for the copier to hold it."""
    until = time.monotonic() + PATIENCE
    while time.monotonic() < until:
        pasted = subprocess.run(
            ["wl-paste", "--no-newline", "--type", TEXT],
            capture_output=True,
            check=False,
        )
        if pasted.stdout == held.encode():
            return True
        time.sleep(0.2)
    return False


def ended(copier: subprocess.Popen[bytes]) -> str | None:
    """Say a copier has ended, so nothing of its memory can be judged, if it has."""
    code = copier.poll()
    return None if code is None else f"it ended with {code} while it should hold the copy"


def readable(pid: int) -> str | None:
    """Say how the account's other processes could read a process, if they could."""
    owner = Path(f"/proc/{pid}/mem").stat().st_uid
    if owner == os.getuid():
        return "/proc/<pid>/mem is the account's, so it's dumpable"
    return None


def unlocked(pid: int) -> str | None:
    """Say a process holds no locked memory, if it holds none."""
    for line in Path(f"/proc/{pid}/status").read_text().splitlines():
        if line.startswith("VmLck:"):
            return None if int(line.split()[1]) > 0 else "VmLck is 0 kB"
    return "its status names no VmLck"


def ended_within(copier: subprocess.Popen[bytes]) -> int | None:
    """Answer how the copier ended, or None, ended here, if it held on."""
    try:
        return copier.wait(timeout=PATIENCE)
    except subprocess.TimeoutExpired:
        copier.kill()
        copier.wait()
        return None


def refusal(ended: int | None) -> str | None:
    """Say how a copier that should have refused didn't, if it didn't."""
    if ended is None:
        return "it held the clipboard"
    if ended == 0:
        return "it ended as if it had copied"
    return None


def main() -> None:
    """Run every case, and fail naming each that didn't hold."""
    with tempfile.TemporaryDirectory() as folder:
        cases = Cases(sys.argv[1], sys.argv[2], Path(folder))
        cases.copied()
        cases.refused()
    if cases.failures:
        for failure in cases.failures:
            print(failure, file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
