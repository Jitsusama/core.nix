"""Copies secrets and ordinary text on niri, and checks a sandbox's window is offered no secret.

A fenced command's windows open on pi-fence-display's wp-security-context-v1 socket, where niri
offers no data control, but a window there that has the keyboard is offered the selection
through wl_data_device and the primary selection, as any focused window is. A copy a password
manager marks secret, as wl-copy --sensitive does with x-kde-passwordManagerHint, must reach no
such window (keeps-secret-copies-from-sandboxes.patch), while ordinary text still does and
Joel's own windows are offered both, as before. This runs pi-fence-display on the nested niri,
copies with wl-copy on Joel's display as a password manager does, and checks:

1. Ordinary text copied while a sandbox's window has the focus is offered to it, and reads.
2. A secret copied then is offered to it as nothing it can read.
3. Ordinary text copied after the secret is offered to it again, and reads.
4. A sandbox's window taking the focus while a secret is held is offered nothing it can read.
5. The primary selection holds to the same: a secret is withheld, ordinary text reaches it.
6. Joel's window is offered a secret, marked as one, and reads it, in both selections.

Every case is checked and reported before the stand-in fails.

usage: keeps-secret-copies-from-sandboxes.py <pi-fence-display>
"""

import os
import secrets
import select
import struct
import subprocess
import sys
import time
from pathlib import Path

from stand_in import PATIENCE, Connection, ask, opened, settle
from wayland_wire import read_string, string

# The type a password manager marks a copy with, as wl-copy --sensitive offers it.
HINT = "x-kde-passwordManagerHint"
TEXT = "text/plain;charset=utf-8"
# The selection protocols each stand-in binds, at most at these versions.
SELECTIONS = {"wl_data_device_manager": 3, "zwp_primary_selection_device_manager_v1": 1}
# How long a read of an offer is given: one whose source is gone is never written.
READING = 2.0


class Selection:
    """One selection of one stand-in: its device, and how its events and requests are numbered."""

    def __init__(self, connection: Connection, manager: str, selected: int, receive: int) -> None:
        """Make the stand-in's device on its seat from `manager`.

        `selected` is the device's selection event's opcode and `receive` its offers' receive
        request's: wl_data_device and the primary selection's device number them differently.
        """
        self.connection = connection
        self.device = connection.made()
        self.selected = selected
        self.receive = receive
        seat = connection.bound["wl_seat"]
        connection.send(connection.bound[manager], 1, struct.pack("<II", self.device, seat))
        connection.roundtrip()

    def since(self, mark: int) -> tuple[list[list[str]], int | None]:
        """Answer every offer made since `mark`, by its types, and the offer selected last."""
        types: dict[int, list[str]] = {}
        selected: int | None = None
        for source, opcode, payload in self.connection.heard[mark:]:
            if source == self.device and opcode == 0:
                types[struct.unpack_from("<I", payload)[0]] = []
            elif source in types and opcode == 0:
                types[source].append(read_string(payload, 0)[0])
            elif source == self.device and opcode == self.selected:
                selected = struct.unpack_from("<I", payload)[0] or None
        return list(types.values()), selected

    def selected_offer(self) -> int | None:
        """Answer the offer selected last since the stand-in connected, if one is."""
        return self.since(0)[1]

    def read(self, offer: int, kind: str = TEXT) -> bytes:
        """Read an offer as `kind`, answering nothing where it's never written in time."""
        reading, writing = os.pipe()
        self.connection.send_fd(offer, self.receive, string(kind), writing)
        os.close(writing)
        read = b""
        until = time.monotonic() + READING
        try:
            while (left := until - time.monotonic()) > 0:
                if not select.select([reading], [], [], left)[0]:
                    break
                chunk = os.read(reading, 4096)
                if not chunk:
                    break
                read += chunk
        finally:
            os.close(reading)
        return read


class StandIn:
    """A stand-in with a window and both selections, Joel's or a sandbox's."""

    def __init__(self, path: Path, app_id: str) -> None:
        """Connect at `path`, open a window with this app ID, and make both selection devices."""
        self.connection = Connection(path, SELECTIONS)
        self.connection.window(app_id)
        self.clipboard = Selection(self.connection, "wl_data_device_manager", 5, 1)
        self.primary = Selection(self.connection, "zwp_primary_selection_device_manager_v1", 1, 0)
        self.window = int(opened(app_id, self.connection)["id"])


def ordinary() -> str:
    """Make ordinary text no earlier copy held."""
    return f"ordinary-{secrets.token_hex(4)}"


def secret() -> str:
    """Make a secret no earlier copy held."""
    return f"secret-{secrets.token_hex(4)}"


def withheld(selection: Selection, mark: int, held: str) -> str | None:
    """Say how a secret reached a selection since `mark`, if it did."""
    offers, _ = selection.since(mark)
    if any(HINT in types for types in offers):
        return "it was offered, marked as a secret"
    offer = selection.selected_offer()
    if offer is not None and selection.read(offer) == held.encode():
        return "its selected offer read as the secret"
    return None


def reaches(selection: Selection, text: str) -> str | None:
    """Say how text failed to reach a selection, if it did."""
    offer = selection.selected_offer()
    if offer is None:
        return "nothing is selected"
    read = selection.read(offer)
    return None if read == text.encode() else f"its selected offer read as {read!r}"


class Cases:
    """The six cases, run on Joel's stand-in and a sandbox's, with what failed in each."""

    def __init__(self, joel: StandIn, sandbox: StandIn) -> None:
        """Run them against these two stand-ins."""
        self.joel = joel
        self.sandbox = sandbox
        self.failures: list[str] = []

    def check(self, case: str, failed: str | None) -> None:
        """Keep a case's failure, or say it held."""
        if failed is None:
            print(f"{case}: held")
        else:
            self.failures.append(f"{case}: {failed}")

    def copy(self, *words: str) -> None:
        """Copy on Joel's display as a password manager does, through wl-copy and data control."""
        subprocess.run(["wl-copy", *words], check=True)
        settle(self.joel.connection, self.sandbox.connection)

    def focus(self, stand_in: StandIn) -> None:
        """Give a stand-in's window the keyboard by its id, as Joel's launcher does."""
        ask({"Action": {"FocusWindow": {"id": stand_in.window}}})
        settle(self.joel.connection, self.sandbox.connection)

    def a_sandbox_focused(self) -> None:
        """Run cases 1 to 3: copies made while the sandbox's window has the focus."""
        sandbox = self.sandbox.clipboard
        self.focus(self.sandbox)
        text = ordinary()
        self.copy(text)
        self.check("ordinary text copied while a sandbox has the focus", reaches(sandbox, text))

        held = secret()
        mark = len(self.sandbox.connection.heard)
        self.copy("--sensitive", held)
        self.check("a secret copied while a sandbox has the focus", withheld(sandbox, mark, held))

        text = ordinary()
        self.copy(text)
        self.check("ordinary text copied after a secret", reaches(sandbox, text))

    def a_sandbox_taking_the_focus(self) -> None:
        """Run case 4: the sandbox's window taking the focus while a secret is held."""
        self.focus(self.joel)
        held = secret()
        self.copy("--sensitive", held)
        mark = len(self.sandbox.connection.heard)
        self.focus(self.sandbox)
        failed = withheld(self.sandbox.clipboard, mark, held)
        self.check("a sandbox taking the focus while a secret is held", failed)

    def the_primary_selection(self) -> None:
        """Run case 5: the primary selection, while the sandbox's window has the focus."""
        sandbox = self.sandbox.primary
        held = secret()
        mark = len(self.sandbox.connection.heard)
        self.copy("--primary", "--sensitive", held)
        self.check("a secret in the primary selection", withheld(sandbox, mark, held))
        text = ordinary()
        self.copy("--primary", text)
        self.check("ordinary text in the primary selection", reaches(sandbox, text))

    def joels_window(self) -> None:
        """Run case 6: Joel's own window offered secrets, in both selections, as before."""
        self.focus(self.joel)
        for selection, first in ((self.joel.clipboard, ()), (self.joel.primary, ("--primary",))):
            held = secret()
            mark = len(self.joel.connection.heard)
            self.copy(*first, "--sensitive", held)
            offers, _ = selection.since(mark)
            case = f"Joel's window offered a secret{' as primary' if first else ''}"
            if not any(HINT in types for types in offers):
                self.check(case, "no offer was marked as a secret")
            else:
                self.check(case, reaches(selection, held))


def main() -> None:
    """Run the six cases against pi-fence-display's socket, and fail on any that didn't hold."""
    runtime = Path(os.environ["XDG_RUNTIME_DIR"])
    filtered = runtime / "pi-fence-wayland"
    with subprocess.Popen([sys.argv[1], "--socket", str(filtered)]) as display:
        try:
            until = time.monotonic() + PATIENCE
            while not filtered.is_socket():
                if time.monotonic() > until or display.poll() is not None:
                    sys.exit("pi-fence-display never opened its socket")
                # A twentieth of a second at a time, since nothing says when it's bound.
                time.sleep(0.05)
            joel = StandIn(runtime / os.environ["WAYLAND_DISPLAY"], "joel.window")
            cases = Cases(joel, StandIn(filtered, "pi.fence.window"))
            cases.a_sandbox_focused()
            cases.a_sandbox_taking_the_focus()
            cases.the_primary_selection()
            cases.joels_window()
        finally:
            display.terminate()
    for failure in cases.failures:
        print(failure, file=sys.stderr)
    if cases.failures:
        sys.exit(f"{len(cases.failures)} copies went where they shouldn't, or never arrived")
    print("no secret reached a sandbox's window, and every ordinary copy and Joel's did")


if __name__ == "__main__":
    main()
