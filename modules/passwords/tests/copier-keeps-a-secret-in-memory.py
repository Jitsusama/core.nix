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
6. A standard input it can't read is refused for that reason, making nothing under TMPDIR and
   running nothing.
7. A standard input past the copier's cap is refused for being past it, rather than held in
   part, so a refusal for any other reason, such as memory it couldn't lock, fails here.
8. A secret long enough to outgrow the copier's first page pastes whole, making nothing under
   TMPDIR, running nothing and staying locked in memory once grown. It's copied with no type
   given, where stock wl-copy asks xdg-mime for one, so case 3's xdg-mime has a copy to run on.
9. It says it's ready before it's given the secret, and by then can't be read through /proc.
10. It says it holds the copy once it's given it.
11. A copy it refuses is never said to be held.
12. The copy is offered marked secret, as x-kde-passwordManagerHint, the mark niri, the
    clipboard door and the clipboard's history each keep it from others by.

Every case is checked and reported before it fails. Cases 7 and 8 lock past a MiB at once, as
the copier grows its buffer, so the limit on locked memory is raised to its hard limit first,
and one too low for them fails naming it rather than as a refusal or a paste that never came.

usage: copier-keeps-a-secret-in-memory.py <copier> <shell for the stand-ins>
"""

import contextlib
import ctypes
import errno
import os
import resource
import secrets
import select
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
# A secret's length past the copier's first page of 4 KiB, so its buffer grows several times.
LONG = 64 * 1024
# What the copier says as it refuses, and why, for an unreadable stdin and one past its cap.
REFUSED = "wl-copy: refused a sensitive copy: "
UNREADABLE = REFUSED + os.strerror(errno.EISDIR)
PAST_THE_CAP = REFUSED + "it is past 1 MiB"
# The mark of a secret copy, which every wall keeping one from others looks for.
HINT = "x-kde-passwordManagerHint"
# The locked memory cases 7 and 8 need, in bytes: a MiB and its page, and the half before it.
LOCKS = 2 * 1024 * 1024

LIBC = ctypes.CDLL(None, use_errno=True)


class Cases:
    """The twelve cases, run against one copier, with how each came out."""

    def __init__(self, copier: str, shell: str, folder: Path) -> None:
        """Run them against `copier`, its stand-ins run by `shell` and its TMPDIR in `folder`."""
        self.copier = copier
        self.outcomes: list[tuple[str, str | None]] = []
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
        """Note how a case came out, its failure or None, to be told once every case has run."""
        self.outcomes.append((case, failed))

    def copier_started(self, stdin: int, *, typed: bool = True) -> subprocess.Popen[bytes]:
        """Start the copier as the daemon does, reading from `stdin`, keeping what it says.

        Untyped, it's given no type, which the daemon always gives.
        """
        typing = ["--type", TEXT] if typed else []
        return subprocess.Popen(
            [self.copier, "--sensitive", "--say-held", "--foreground", *typing],
            stdin=stdin,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
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
        """Run cases 1 to 5, 9, 10 and 12: a secret copied and pasted, and the copier holding it."""
        held = f"secret-{secrets.token_hex(8)}"
        reading, writing = os.pipe()
        copier = self.copier_started(reading)
        os.close(reading)
        said = said_next(copier)
        unready = None if said == "ready" else f"it said {said!r} rather than ready"
        self.check("it says it's ready first", unready or readable(copier.pid))
        given(writing, held)
        try:
            pasted = pasted_once_held(held)
            self.check("the secret pastes whole", None if pasted else "it never pasted")
            self.check("it's offered marked secret", ended(copier) or unmarked())
            said = said_next(copier)
            unheld = None if said == "held" else f"it said {said!r} rather than held"
            self.check("it says it holds the copy", unheld)
            self.check("nothing is made under TMPDIR", self.anything_made())
            self.check("nothing on PATH runs", self.anything_ran())
            self.check("it can't be read through /proc", ended(copier) or readable(copier.pid))
            self.check("its secret is locked in memory", ended(copier) or unlocked(copier.pid))
        finally:
            copier.kill()
            copier.wait()

    def copied_long(self) -> None:
        """Run case 8: a secret past the copier's first page, pasted whole and held as short."""
        held = secrets.token_hex(LONG // 2)
        reading, writing = os.pipe()
        copier = self.copier_started(reading, typed=False)
        os.close(reading)
        given(writing, held)
        try:
            pasted = pasted_once_held(held)
            self.check("a 64 KiB secret pastes whole", None if pasted else "it never pasted")
            self.check("a 64 KiB secret makes nothing under TMPDIR", self.anything_made())
            self.check("a 64 KiB secret runs nothing", self.anything_ran())
            self.check("a grown secret is locked", ended(copier) or unlocked(copier.pid))
        finally:
            copier.kill()
            copier.wait()

    def refused(self) -> None:
        """Run cases 6 and 7: a standard input it can't read, then one past its cap."""
        unreadable = os.open(self.made.parent, os.O_RDONLY | os.O_DIRECTORY)
        copier = self.copier_started(unreadable)
        os.close(unreadable)
        ended, said, told = ended_within(copier)
        failed = refusal(ended, said, UNREADABLE)
        self.check("an unreadable stdin is refused", failed or self.anything_made())
        self.check("an unreadable stdin runs nothing", self.anything_ran())
        self.check("an unreadable stdin is never said held", said_held(told))

        reading, writing = os.pipe()
        copier = self.copier_started(reading)
        os.close(reading)
        with contextlib.suppress(BrokenPipeError):
            os.write(writing, b"x" * TOO_LONG)
        os.close(writing)
        ended, said, told = ended_within(copier)
        failed = refusal(ended, said, PAST_THE_CAP)
        self.check("a stdin past the cap is refused", failed or self.anything_made())
        self.check("a stdin past the cap is never said held", said_held(told))


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


def unmarked() -> str | None:
    """Say what Joel's display is offered in place of a copy marked secret, if it isn't marked."""
    listed = subprocess.run(["wl-paste", "--list-types"], capture_output=True, check=False)
    offered = listed.stdout.decode(errors="replace").split()
    return None if HINT in offered else f"it's offered as {offered}, not marked {HINT}"


def locks_enough() -> str | None:
    """Raise the limit on locked memory to its hard limit, saying so if that's too low."""
    _, hard = resource.getrlimit(resource.RLIMIT_MEMLOCK)
    resource.setrlimit(resource.RLIMIT_MEMLOCK, (hard, hard))
    if hard != resource.RLIM_INFINITY and hard < LOCKS:
        return f"only {hard} bytes may be locked, under the {LOCKS} cases 7 and 8 need"
    return None


def given(writing: int, held: str) -> None:
    """Give the copier its secret, leaving one that has already ended to fail the checks after."""
    with contextlib.suppress(BrokenPipeError):
        os.write(writing, held.encode())
    os.close(writing)


def said_next(copier: subprocess.Popen[bytes]) -> str:
    """Answer the next line the copier says on its standard output, or none if none comes."""
    if copier.stdout is None or not select.select([copier.stdout], [], [], PATIENCE)[0]:
        return ""
    line: bytes = copier.stdout.readline()
    return line.decode(errors="replace").strip()


def said_held(told: str) -> str | None:
    """Say a refusing copier said it held the copy, if it said so, as it may say only ready."""
    return None if told.split() in ([], ["ready"]) else f"it said {told.split()}"


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


def ended_within(copier: subprocess.Popen[bytes]) -> tuple[int | None, str, str]:
    """Answer how the copier ended, or None, ended here, if it held on, and what it said.

    What it said is on its standard error, then its standard output.
    """
    try:
        told, said = copier.communicate(timeout=PATIENCE)
    except subprocess.TimeoutExpired:
        copier.kill()
        told, said = copier.communicate()
        return None, said.decode(errors="replace"), told.decode(errors="replace")
    return copier.returncode, said.decode(errors="replace"), told.decode(errors="replace")


def refusal(ended: int | None, said: str, why: str) -> str | None:
    """Say how a copier that should have refused for `why` didn't, if it didn't."""
    if ended is None:
        return "it held the clipboard"
    if ended == 0:
        return "it ended as if it had copied"
    if why not in said:
        return f"it ended with {ended}, saying {said.strip()!r} rather than {why!r}"
    return None


def main() -> None:
    """Run every case, and fail naming each that didn't hold."""
    with tempfile.TemporaryDirectory() as folder:
        cases = Cases(sys.argv[1], sys.argv[2], Path(folder))
        cases.check("enough memory may be locked", locks_enough())
        cases.copied()
        cases.copied_long()
        cases.refused()
    for case, failed in cases.outcomes:
        print(f"{case}: {failed or 'held'}", file=sys.stderr if failed else sys.stdout)
    if any(failed for _, failed in cases.outcomes):
        sys.exit(1)


if __name__ == "__main__":
    main()
