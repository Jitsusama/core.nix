"""Shows a layer surface's blur taking back its textures when it opens again, not making them.

The launcher's layer surface stays mapped while it's shut, with an empty blur region, so niri
draws no blur for it then, and smithay lets go of the textures that blur was drawn through. When
the launcher opened again, niri made its framebuffer and four blur textures afresh inside the
frame that showed it, 22 to 48 ms of that frame on Joel's screen under load. niri now keeps what
a blur let go and takes it back for the next of its size (blur-kept-for-its-size.patch).

The stand-in is an overlay layer surface named as the launcher is, which the account's layer rule
blurs. It opens with a blur region over all of it, shuts by an empty one, opens again, grows,
goes back to its first size and draws once more as it is, each step waiting for niri to draw it.
It takes the grown size's width at another height, then its area turned on its side, while the
grown set is kept, so a pool matching on width or on bytes alone would hand that set out, and
takes the grown size back to show that set was kept. Then it nearly fills the screen at fourteen
sizes in turn, more than the pool's 64 MiB keeps, going back to the second after five and again
after the last. niri says at debug what each blur's
textures came from, and each step is judged by what niri said after it alone: the first opening
makes its set, shutting and a frame that changes nothing make none, opening again takes the whole
set back, growing gives the set up for the next blur of its size, which going back takes, a size
sharing only a width or an area with a kept set makes its own, the walk's second set is still
kept after three newer sizes and once more after all of them, and the pool never says it keeps
more than 64 MiB once it has let go.

usage: keeps-a-blur-ready.py <niri's log>
"""

import os
import re
import struct
import sys
import time
from pathlib import Path

from stand_in import PATIENCE, SETTLE, Connection
from wayland_wire import string

# The surface's first size, the size it grows to, and the overlay layer it asks for, anchored
# nowhere, so niri centres it. Both sizes fit the stand-in's pool.
FIRST = (400, 300)
GROWN = (480, 300)
# The grown size's width at another height, as the launcher's card and drawer are, and its area
# turned on its side: neither may take the grown set.
TALLER = (480, 360)
TURNED = (300, 480)
# The nested screen, what sway's frame leaves of it.
SCREEN = (1276, 773)
# Sizes inside the nested screen, so niri cuts none to fit.
# Each set, a framebuffer and a blur's textures from its full size down to an eighth, is 6.6 to
# 7.5 MiB, in the pool's own unit, and the pool lets go a texture at a time, oldest first.
FILLING = [(1200 - 10 * step, 700) for step in range(14)]
# After the walk's fifth size it goes back to its second, which the third, fourth and fifth
# followed: taking it back needs a pool of about 22 MiB, so one too small for the launcher's 21
# MiB set on Joel's screen has let it go.
RECALLED_AFTER = 5
# The pool's cap, in whole MiB as niri says it. The pool lets go a texture at a time until it holds
# no more, so what it keeps once it has let go is within a texture, 3.2 MiB at most here, of its
# cap: saying 64 or less holds the cap under about 68 MiB, short of the 69 the order step needs.
MOST = 64
OVERLAY = 3
# The wl_shm format with alpha, as the launcher draws: smithay skips drawing whatever an opaque
# surface covers, the blur behind it included, so an opaque stand-in would never be blurred.
ARGB8888 = 0
# The namespace the account's layer rule blurs behind, without seeing through to the wallpaper.
NAMESPACE = "launcher"
# niri's line for a blur's textures, its colours taken out.
SAID = re.compile(
    r"a background effect sized (\d+) \S+ (\d+) made (\d+) textures and took (\d+) kept"
)
# niri's line for textures the pool let go, printed beside the step that made it say so, since
# that tells a cap too small from an effect keeping its old set before taking its new one.
LET_GO = re.compile(r"the texture pool let go of \d+ textures, keeping (\d+) MB")
COLOURS = re.compile(r"\x1b\[[0-9;]*m")


class Launcher:
    """One blurred layer surface on a stand-in's connection, whose size and blur it sets."""

    def __init__(self, connection: Connection) -> None:
        """Make the layer surface and its effect, answer its first configure, and give it a buffer.

        Nothing is drawn until the first blur is set, which commits the buffer with it.
        """
        self.connection = connection
        self.surface = connection.made()
        self.role = connection.made()
        self.effect = connection.made()
        self.size = FIRST
        compositor = connection.bound["wl_compositor"]
        connection.send(compositor, 0, struct.pack("<I", self.surface))
        made = struct.pack("<III", self.role, self.surface, 0)
        connection.send(
            connection.bound["zwlr_layer_shell_v1"],
            0,
            made + struct.pack("<I", OVERLAY) + string(NAMESPACE),
        )
        effects = connection.bound["ext_background_effect_manager_v1"]
        connection.send(effects, 1, struct.pack("<II", self.effect, self.surface))
        self.resize(FIRST)

    def resize(self, size: tuple[int, int]) -> None:
        """Ask for `size`, answer the configure niri sends for it, and attach a buffer of it.

        niri may draw a frame between asking and attaching, the old buffer and its blur at the new
        size's place, centred. Where that runs off the screen, niri blurs the cut size and makes a
        set for it, so each size must leave room at its place for the one before.
        """
        connection = self.connection
        width, height = size
        if any(
            (whole - new) // 2 + old > whole
            for whole, new, old in zip(SCREEN, size, self.size, strict=True)
        ):
            sys.exit(f"the walk goes from {self.size} to {size}, so the old runs off the screen")
        connection.send(self.role, 0, struct.pack("<II", width, height))
        connection.send(self.surface, 6)
        opcode, configured = connection.wait_for(self.role, "the layer surface's configure")
        # A layer surface's closed is its event 1, and its configure event 0.
        if opcode != 0:
            sys.exit("niri closed the stand-in's layer surface")
        connection.send(self.role, 6, configured[:4])
        buffer = connection.made()
        connection.send(
            connection.pool,
            0,
            struct.pack("<IiiiiI", buffer, 0, width, height, width * 4, ARGB8888),
        )
        connection.send(self.surface, 1, struct.pack("<Iii", buffer, 0, 0))
        self.size = size

    def blur(self, *, over: bool) -> None:
        """Set the blur region over the whole surface or over none of it, and wait for a frame.

        The region is double-buffered, so it takes with the commit, and the frame callback
        asked for with it is done only once niri has drawn the surface as it now is.
        """
        connection = self.connection
        width, height = self.size
        region = connection.made()
        connection.send(connection.bound["wl_compositor"], 1, struct.pack("<I", region))
        if over:
            connection.send(region, 1, struct.pack("<iiii", 0, 0, width, height))
        connection.send(self.effect, 1, struct.pack("<I", region))
        connection.send(region, 0)
        callback = connection.made()
        connection.send(self.surface, 3, struct.pack("<I", callback))
        connection.send(self.surface, 9, struct.pack("<iiii", 0, 0, width, height))
        connection.send(self.surface, 6)
        connection.wait_for(callback, "the frame that drew the stand-in's surface")


def said(log: Path) -> list[tuple[int, int, int, int]]:
    """Each blur's size, how many textures it made, and how many it took kept, in order."""
    return [
        (int(width), int(height), int(made), int(kept))
        for width, height, made, kept in SAID.findall(read(log))
    ]


def read(log: Path) -> str:
    """Niri's log, its colours taken out."""
    return COLOURS.sub("", log.read_text(encoding="utf-8", errors="replace"))


class Steps:
    """What niri said of blurs' textures after each step, apart from what it said before."""

    def __init__(self, log: Path) -> None:
        """Read niri's log at `log`, from what it holds now."""
        self.log = log
        self.seen = len(said(log))
        self.let_go = len(LET_GO.findall(read(log)))
        self.heard_let_go = False

    def after(self, name: str, size: tuple[int, int], *, says: bool) -> tuple[int, int]:
        """How many textures niri made and took kept since the last step, adding up its lines.

        A step niri should say something after waits for its first line; every step then gives
        niri a moment more, so a second line from the same step is counted with it. Every line
        has to be of the surface's `size`, so no other blur's are counted as its.
        """
        until = time.monotonic() + PATIENCE
        while says and len(said(self.log)) == self.seen:
            if time.monotonic() > until:
                # Once a step has been heard, the patch and the log level are known good.
                why = (
                    "niri kept the effect itself, or never drew it"
                    if self.seen
                    else "is blur-kept-for-its-size.patch applied, and niri logging at debug?"
                )
                sys.exit(
                    f"niri said nothing of where a blur's textures came from after {name}: {why}"
                )
            time.sleep(0.05)
        time.sleep(SETTLE)
        lines = said(self.log)[self.seen :]
        self.seen += len(lines)
        let_go = list(LET_GO.finditer(read(self.log)))[self.let_go :]
        self.let_go += len(let_go)
        for line in let_go:
            print(f"{name}: {line[0]}")
            self.heard_let_go = True
            if int(line[1]) > MOST:
                sys.exit(f"after {name}, the pool kept {line[1]} MB: its cap is over {MOST} MiB")
        for width, height, made, kept in lines:
            print(f"{name}: a blur {width} x {height} made {made} and took {kept} kept")
            if (width, height) != size:
                sys.exit(f"after {name}, niri spoke of a blur {width} x {height}, not {size}")
        return sum(line[2] for line in lines), sum(line[3] for line in lines)


def main() -> None:
    """Open, shut, open, resize and fill with the blurred surface, failing on any wrong step."""
    log = Path(sys.argv[1])
    path = Path(os.environ["XDG_RUNTIME_DIR"]) / os.environ["WAYLAND_DISPLAY"]
    connection = Connection(
        path, also={"zwlr_layer_shell_v1": 3, "ext_background_effect_manager_v1": 1}
    )
    steps = Steps(log)
    launcher = Launcher(connection)

    launcher.blur(over=True)
    made, kept = steps.after("opening", FIRST, says=True)
    # The framebuffer is one; a set of one would have no blur of its own to keep.
    if made < 2 or kept != 0:  # noqa: PLR2004
        sys.exit(f"the first opening made {made} textures and took {kept}, not a whole set")
    whole = made

    launcher.blur(over=False)
    if steps.after("shutting", FIRST, says=False) != (0, 0):
        sys.exit("shutting the blur made or took textures, where it should only let them go")

    launcher.blur(over=True)
    again = steps.after("opening again", FIRST, says=True)
    if again != (0, whole):
        sys.exit(
            f"opening again made {again[0]} textures and took {again[1]} kept, not {whole} kept: "
            "niri let go of what its blur was drawn through"
        )

    launcher.resize(GROWN)
    launcher.blur(over=True)
    grown = steps.after("growing", GROWN, says=True)
    if grown != (whole, 0):
        sys.exit(f"growing made {grown[0]} textures and took {grown[1]}, not a new set of {whole}")

    launcher.resize(FIRST)
    launcher.blur(over=True)
    back = steps.after("going back", FIRST, says=True)
    if back != (0, whole):
        sys.exit(
            f"going back made {back[0]} textures and took {back[1]} kept, not {whole} kept: "
            "niri let go of what a blur gave up when its size changed"
        )

    launcher.blur(over=True)
    if steps.after("drawing again", FIRST, says=False) != (0, 0):
        sys.exit("a frame that changed nothing made or took textures: the blur remakes its set")

    alike(launcher, steps, whole)

    for size in FILLING[:RECALLED_AFTER]:
        fill(launcher, steps, size, expect=(whole, 0), why="a size the pool had never seen")
    fill(
        launcher,
        steps,
        FILLING[1],
        expect=(0, whole),
        why="the pool let go of a set three newer ones followed: it keeps too little for one",
    )
    for size in FILLING[RECALLED_AFTER:]:
        fill(launcher, steps, size, expect=(whole, 0), why="a size the pool had never seen")
    # After the last size it goes back to the second once more. That set is then the oldest
    # the pool holds whole, about 63 MiB with the eight newer ones, so a pool under that has let
    # some of it go. Under about 69 MiB, so has one keeping the last size's set before taking
    # the second's, which pushes out the very set about to be taken, and what the pool says it
    # keeps holds its cap under about 68. About 1 MiB is to spare at 64 MiB, so sets grown by
    # another niri fail here too.
    fill(
        launcher,
        steps,
        FILLING[1],
        expect=(0, whole),
        why="the pool let go of the oldest set it held whole: it keeps less than about 63 MiB, "
        "or kept the set given up before taking this one, or each set grew",
    )
    if not steps.heard_let_go:
        sys.exit(
            "niri never said its pool let go of textures: its cap is over about 97 MiB, "
            "or its let-go line no longer reads as LET_GO expects"
        )


def alike(launcher: Launcher, steps: Steps, whole: int) -> None:
    """Fail unless sizes sharing only a width or an area with the kept grown set make their own.

    Taking the grown size back afterwards shows its set was kept all along, so the two before it
    were offered it and refused it.
    """
    for name, size, like in (
        ("taking another height", TALLER, "its width at another height"),
        ("turning on its side", TURNED, "its area on its side"),
    ):
        launcher.resize(size)
        launcher.blur(over=True)
        made, kept = steps.after(name, size, says=True)
        if (made, kept) != (whole, 0):
            sys.exit(
                f"{name} made {made} textures and took {kept} kept, not a new set of {whole}: "
                f"the pool handed out a set of {like}"
            )
    launcher.resize(GROWN)
    launcher.blur(over=True)
    made, kept = steps.after("growing again", GROWN, says=True)
    if (made, kept) != (0, whole):
        sys.exit(
            f"growing again made {made} textures and took {kept} kept, not {whole} kept: the "
            "pool let the grown set go, so the two sizes before it were never offered it"
        )


def fill(
    launcher: Launcher,
    steps: Steps,
    size: tuple[int, int],
    *,
    expect: tuple[int, int],
    why: str,
) -> None:
    """Resize and blur the surface at a size of the walk, failing with `why` if not `expect`."""
    launcher.resize(size)
    launcher.blur(over=True)
    name = f"filling at {size[0]} x {size[1]}"
    filled = steps.after(name, size, says=True)
    if filled != expect:
        sys.exit(
            f"{name} made {filled[0]} textures and took {filled[1]} kept, not "
            f"{expect[0]} made and {expect[1]} kept: {why}"
        )


if __name__ == "__main__":
    main()
