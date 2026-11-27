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
5. The copier's memory holding the secret is locked, so it never reaches swap: as much of it
   locked as the secret is long, so a first page locked alone doesn't pass for it.
6. A standard input it can't read is refused for that reason, making nothing under TMPDIR and
   running nothing.
7. A standard input a byte past the copier's cap of a MiB is refused for being past it, rather
   than held in part, so a refusal for any other reason, such as memory it couldn't lock, fails
   here; it makes nothing under TMPDIR and runs nothing either.
8. A secret long enough to outgrow the copier's first page pastes whole, making nothing under
   TMPDIR, running nothing and staying locked in memory once grown. It's copied with no type
   given, where stock wl-copy asks xdg-mime for one, so case 3's xdg-mime has a copy to run on.
9. It says it's ready before it's given the secret, and by then can't be read through /proc.
10. It says it holds the copy once it's given it, and by then Joel's display has it: read
    before any paste, one paste at once pastes it whole, as the daemon answers copied then.
11. A copy it refuses is never said to be held.
12. The copy is offered marked secret, as x-kde-passwordManagerHint, the mark niri, the
    clipboard door and the clipboard's history each keep it from others by.
13. A secret of exactly the cap pastes whole and stays locked in memory, so only a byte past it
    is refused: with case 7, it pins the edge the copier grows its buffer a byte past the cap
    to tell. A copier that refuses it instead is failed with what it said as it ended.
14. Told to say it holds a copy that isn't sensitive, it ends saying so, never ready or held,
    making nothing under TMPDIR and running nothing, as only a sensitive copy says either.

Every case is checked and reported before it fails. Where a copy in case 1, 8 or 13 never
pastes, a copier that ended is failed with how it ended and what it said. Cases 7 and 13 lock
past 2 MiB at once, as the copier grows its buffer, so the limit on locked memory is raised to
its hard limit first, and one too low for them fails naming it rather than as a refusal or a
paste that never came.

usage: copier-keeps-a-secret-in-memory.py <copier> <shell for the stand-ins>
"""

import contextlib
import ctypes
import errno
import io
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
# The copier's cap on a sensitive copy, of a MiB, in bytes (SENSITIVE_CAP in the patch).
CAP = 1024 * 1024
# A secret's length past the copier's first page of 4 KiB, so its buffer grows several times.
LONG = 64 * 1024
# What the copier says as it refuses, and why, for an unreadable stdin and one past its cap.
REFUSED = "wl-copy: refused a sensitive copy: "
UNREADABLE = REFUSED + os.strerror(errno.EISDIR)
PAST_THE_CAP = REFUSED + "it is past 1 MiB"
# What it says, ending, when told to say ready and held for a copy that isn't sensitive.
SENSITIVE_ONLY = "--say-held needs --sensitive"
# The mark of a secret copy, which every wall keeping one from others looks for.
HINT = "x-kde-passwordManagerHint"
# The locked memory cases 7 and 13 need, in bytes: as the buffer grows from the cap to a byte
# past it, the cap's pages and the next's, a MiB and a byte rounded up to its page, at once.
LOCKS = 2 * CAP + 4096

LIBC = ctypes.CDLL(None, use_errno=True)

# What each copier said past the line said_next last answered of it, kept for its next call.
UNANSWERED: dict[subprocess.Popen[bytes], bytes] = {}


class Cases:
    """The fourteen cases, run against one copier, with how each came out."""

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
            said = said_next(copier)
            # Pasted the moment it says held, before any other step, so a held said early has
            # the least time to catch up. What this can't see is a held said a moment before
            # the display has the copy, if the copier takes it before the paste asks for it.
            once = pasted_now(held) if said == "held" else "it never said held"
            unheld = None if said == "held" else f"it said {said!r} rather than held"
            self.check("it says it holds the copy", unheld)
            self.check("once it says it's held, one paste has it", once)
            pasted = pasted_once_held(held, copier)
            never = None if pasted else refused_instead(copier) or "it never pasted"
            self.check("the secret pastes whole", never)
            self.check("it's offered marked secret", ended(copier) or unmarked())
            self.check("nothing is made under TMPDIR", self.anything_made())
            self.check("nothing on PATH runs", self.anything_ran())
            self.check("it can't be read through /proc", ended(copier) or readable(copier.pid))
            locked = ended(copier) or unlocked(copier.pid, len(held))
            self.check("its secret is locked in memory", locked)
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
            pasted = pasted_once_held(held, copier)
            never = None if pasted else refused_instead(copier) or "it never pasted"
            self.check("a 64 KiB secret pastes whole", never)
            self.check("a 64 KiB secret makes nothing under TMPDIR", self.anything_made())
            self.check("a 64 KiB secret runs nothing", self.anything_ran())
            locked = ended(copier) or unlocked(copier.pid, len(held))
            self.check("a grown secret is locked", locked)
        finally:
            copier.kill()
            copier.wait()

    def copied_at_the_cap(self) -> None:
        """Run case 13: a secret of exactly the cap, pasted whole and locked, not refused."""
        held = secrets.token_hex(CAP // 2)
        reading, writing = os.pipe()
        copier = self.copier_started(reading)
        os.close(reading)
        given(writing, held)
        try:
            pasted = pasted_once_held(held, copier)
            never = None if pasted else refused_instead(copier) or "it never pasted"
            self.check("a secret of exactly the cap pastes whole", never)
            self.check("a secret of exactly the cap makes nothing", self.anything_made())
            self.check("a secret of exactly the cap runs nothing", self.anything_ran())
            locked = ended(copier) or unlocked(copier.pid, len(held))
            self.check("a secret of exactly the cap is locked", locked)
        finally:
            copier.kill()
            copier.wait()

    def refused(self) -> None:
        """Run cases 6 and 7: a standard input it can't read, then one past its cap."""
        unreadable = os.open(self.made.parent, os.O_RDONLY | os.O_DIRECTORY)
        copier = self.copier_started(unreadable)
        os.close(unreadable)
        ended, said, told = ended_within(copier)
        # Whatever was made is asked for and swept even when the refusal failed, so no case
        # after it is handed this one's leftovers.
        made = self.anything_made()
        failed = refusal(ended, said, UNREADABLE)
        self.check("an unreadable stdin is refused", failed or made)
        self.check("an unreadable stdin runs nothing", self.anything_ran())
        self.check("an unreadable stdin is never said held", said_held(told))

        reading, writing = os.pipe()
        copier = self.copier_started(reading)
        os.close(reading)
        stalled = fed(writing, b"x" * (CAP + 1))
        ended, said, told = ended_within(copier)
        made = self.anything_made()
        failed = stalled or refusal(ended, said, PAST_THE_CAP)
        self.check("a stdin a byte past the cap is refused", failed or made)
        self.check("a stdin a byte past the cap runs nothing", self.anything_ran())
        self.check("a stdin a byte past the cap is never said held", said_held(told))

    def unsensitive(self) -> None:
        """Run case 14: told to say it holds a copy that isn't sensitive, it ends saying so."""
        copier = subprocess.Popen(
            [self.copier, "--say-held", "--foreground", "--type", TEXT],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=self.environment,
        )
        ended, said, told = ended_within(copier)
        made = self.anything_made()
        failed = refusal(ended, said, SENSITIVE_ONLY)
        self.check("a copy not sensitive can't be said held", failed or made)
        self.check("a copy not sensitive runs nothing", self.anything_ran())
        quiet = None if told.split() == [] else f"it said {told.split()}"
        self.check("a copy not sensitive is never said ready or held", quiet)


def paste() -> subprocess.CompletedProcess[bytes] | None:
    """Paste once on Joel's display, answering None if wl-paste hadn't ended in time."""
    try:
        pasted = subprocess.run(
            ["wl-paste", "--no-newline", "--type", TEXT],
            capture_output=True,
            check=False,
            timeout=PATIENCE,
        )
    except subprocess.TimeoutExpired:
        return None
    return pasted


def pasted_now(held: str) -> str | None:
    """Say what one paste on Joel's display gave in place of the secret, if not the secret."""
    pasted = paste()
    if pasted is None:
        return f"the paste hadn't ended in {PATIENCE:g} s"
    if pasted.stdout == held.encode():
        return None
    why = pasted.stderr.decode(errors="replace").strip()
    return f"it pasted {len(pasted.stdout)} other bytes, exiting {pasted.returncode}: {why!r}"


def pasted_once_held(held: str, copier: subprocess.Popen[bytes]) -> bool:
    """Answer whether Joel's display pastes the secret, waiting while the copier runs."""
    until = time.monotonic() + PATIENCE
    while time.monotonic() < until:
        pasted = paste()
        if pasted is not None and pasted.stdout == held.encode():
            return True
        if copier.poll() is not None:
            return False
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
        return f"only {hard} bytes may be locked, under the {LOCKS} cases 7 and 13 need"
    return None


def given(writing: int, held: str) -> None:
    """Give the copier its secret, leaving one that ended or stalled to fail the checks after."""
    fed(writing, held.encode())


def fed(writing: int, data: bytes) -> str | None:
    """Write `data` to the copier and close it, saying so if it stopped reading in time.

    A copier that stops reading short of the end but holds its standard input open would
    leave a plain write waiting for it forever; one that ends, closing it, has read its last.
    """
    os.set_blocking(writing, False)
    left = memoryview(data)
    until = time.monotonic() + PATIENCE
    try:
        while left:
            waiting = until - time.monotonic()
            if waiting <= 0 or not select.select([], [writing], [], waiting)[1]:
                return f"it stopped reading with {len(left)} bytes left"
            with contextlib.suppress(BlockingIOError):
                left = left[os.write(writing, left) :]
    except BrokenPipeError:
        return None
    finally:
        os.close(writing)
    return None


def said_next(copier: subprocess.Popen[bytes]) -> str:
    """Answer the next line the copier says on its standard output, or none if none comes.

    The whole read is bounded by PATIENCE from its start: it reads only once select says
    there's something to read, until a newline, the end or the deadline, so a copier that
    says part of a line and holds on is answered what it said, in time, rather than hanging
    the check. What came past the line is kept for the next call on the same copier, as
    two lines said at once come in one read.
    """
    stdout = copier.stdout
    if not isinstance(stdout, io.BufferedReader):
        return ""
    heard = UNANSWERED.pop(copier, b"")
    until = time.monotonic() + PATIENCE
    while b"\n" not in heard:
        waiting = until - time.monotonic()
        if waiting <= 0 or not select.select([stdout], [], [], waiting)[0]:
            break
        more = stdout.read1(65536)
        if not more:
            break
        heard += more
    line, _, rest = heard.partition(b"\n")
    if rest:
        UNANSWERED[copier] = rest
    return line.decode(errors="replace").strip()


def said_held(told: str) -> str | None:
    """Say a refusing copier said it held the copy, if it said so, as it may say only ready."""
    return None if told.split() in ([], ["ready"]) else f"it said {told.split()}"


def ended(copier: subprocess.Popen[bytes]) -> str | None:
    """Say a copier has ended, so nothing of its memory can be judged, if it has."""
    code = copier.poll()
    return None if code is None else f"it ended with {code} while it should hold the copy"


def refused_instead(copier: subprocess.Popen[bytes]) -> str | None:
    """Say how a copier that should hold its copy ended instead, and why, if it ended.

    What it said is read as ended_within reads it, for as long as PATIENCE, since a child it
    left behind could hold its standard error open past its end.
    """
    code = copier.poll()
    if code is None:
        return None
    try:
        _, said = copier.communicate(timeout=PATIENCE)
    except subprocess.TimeoutExpired as held_open:
        copier.kill()
        heard = (held_open.stderr or b"").decode(errors="replace").strip()
        return (
            f"it ended with {code}, its standard error still open after {PATIENCE:g} s, "
            f"having said {heard!r}"
        )
    if code == 0:
        return "it ended with 0, as if it had copied, yet nothing pasted"
    reason = said.decode(errors="replace").strip()
    return f"it ended with {code}, saying {reason!r}"


def readable(pid: int) -> str | None:
    """Say how the account's other processes could read a process, if they could."""
    owner = Path(f"/proc/{pid}/mem").stat().st_uid
    if owner == os.getuid():
        return "/proc/<pid>/mem is the account's, so it's dumpable"
    return None


def unlocked(pid: int, length: int) -> str | None:
    """Say a process holds less locked memory than a secret `length` bytes long, if it does."""
    for line in Path(f"/proc/{pid}/status").read_text().splitlines():
        if line.startswith("VmLck:"):
            locked = int(line.split()[1]) * 1024
            return None if locked >= length else f"VmLck is {locked} bytes, under {length}"
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
        cases.copied_at_the_cap()
        cases.refused()
        cases.unsensitive()
    for case, failed in cases.outcomes:
        print(f"{case}: {failed or 'held'}", file=sys.stderr if failed else sys.stdout)
    if any(failed for _, failed in cases.outcomes):
        sys.exit(1)


if __name__ == "__main__":
    main()
