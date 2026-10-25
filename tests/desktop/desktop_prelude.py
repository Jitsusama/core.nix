"""The head of each desktop area still written as a driver script: the boot and its helpers' names.

tests/desktop.nix runs this, then an area's subtests (desktop-<area>-vm-test.py), as one
script, and its opening comment says what the test proves. The desktop and every helper are
desktop_session.py's, which an area written for pytest asks for as a fixture; this makes one
for the script, starts it, and names its helpers and values as the scripts call them, so both
kinds of area run the same code. It goes once every area is pytest's. run-subtests cuts the
script into subtests at each top-level `with subtest(`, so what's here runs whichever are
chosen.
"""

import json
from pathlib import Path
from typing import TYPE_CHECKING

import desktop_session as session
import vm_helpers

if TYPE_CHECKING:
    from driver_names import machine

# What the scripts name from the session's module, as they named them here.
type Json = session.Json
CAP_SYS_NICE = session.CAP_SYS_NICE
CAPITALS = session.CAPITALS
FOLLOWED_THE_ROOM = session.FOLLOWED_THE_ROOM
FROM_THE_CORNER = session.FROM_THE_CORNER
FROM_THE_EDGE = session.FROM_THE_EDGE
LAUNCHER_OPEN = session.LAUNCHER_OPEN
NEARLY_ALL = session.NEARLY_ALL
PI_PRESENCE = session.PI_PRESENCE
PI_QUEST = session.PI_QUEST
PI_WINDOW = session.PI_WINDOW
PICTURE_COLOURS = session.PICTURE_COLOURS
ROWS_LOOKED_AT = session.ROWS_LOOKED_AT
SIGNING = session.SIGNING
SIGNING_IN = session.SIGNING_IN
STARTING = session.STARTING
changed_at_the_top = session.changed_at_the_top
drawn_at = session.drawn_at
margin_points = session.margin_points
margin_sets = session.margin_sets
settles_on = session.settles_on
margin = session.MARGIN
notices = session.NOTICES

VALUES: session.Values = json.loads(Path("@values@").read_text())
ACCOUNT = VALUES["account"]
SECRET = VALUES["secret"]
ASKING = VALUES["asking"]
ASK_PORTAL = VALUES["ask_portal"]
TOUCHSCREEN = VALUES["touchscreen"]
TUPLE_WINDOW = VALUES["tuple_window"]
PICTURE = VALUES["picture"]
PAGE = VALUES["page"]
TEXT = VALUES["text"]
TRACK = VALUES["track"]
COREUTILS = VALUES["coreutils"]
FFMPEG_HEADLESS = VALUES["ffmpeg_headless"]
FONTCONFIG_BIN = VALUES["fontconfig_bin"]
WEV = VALUES["wev"]
ZENITY = VALUES["zenity"]
ACCENT = VALUES["accent"]
SUNKEN = VALUES["sunken"]
MONO_FAMILY = VALUES["mono_family"]

DESKTOP = session.Desktop(machine, VALUES)

# A busy host is slower at everything, so every wait allows for how busy it is as it starts.
vm_helpers.pace_the_driver(globals(), describe=DESKTOP.described)
print(
    f"waits allow {vm_helpers.pace(vm_helpers.busyness()):.1f} times as long as they say, for now"
)
DESKTOP.start()

answer = DESKTOP.answer
answered = DESKTOP.answered
answered_with = DESKTOP.answered_with
as_account = DESKTOP.as_account
ask = DESKTOP.ask
brightness = DESKTOP.brightness
capture = DESKTOP.capture
captured = DESKTOP.captured
click = DESKTOP.click
colours_and_light = DESKTOP.colours_and_light
commonest_colour_in = DESKTOP.commonest_colour_in
copied_then_closed = DESKTOP.copied_then_closed
focused_kitty = DESKTOP.focused_kitty
held_still = DESKTOP.held_still
in_session = DESKTOP.in_session
launcher_ready = DESKTOP.launcher_ready
launcher_until = DESKTOP.launcher_until
listen = DESKTOP.listen
mark = DESKTOP.mark
meter_shows = DESKTOP.meter_shows
newest_size = DESKTOP.newest_size
notes_window = DESKTOP.notes_window
offer_to_play = DESKTOP.offer_to_play
opens_in = DESKTOP.opens_in
pasted_the_picture = DESKTOP.pasted_the_picture
photograph = DESKTOP.photograph
pi_session = DESKTOP.pi_session
power = DESKTOP.power
presence_says = DESKTOP.presence_says
reported = DESKTOP.reported
screen_now = DESKTOP.screen_now
settled_screen = DESKTOP.settled_screen
shell_said = DESKTOP.shell_said
showing = DESKTOP.showing
step_in = DESKTOP.step_in
tokened = DESKTOP.tokened
told = DESKTOP.told
tuple_said = DESKTOP.tuple_said
type_in = DESKTOP.type_in
wait_for_surface = DESKTOP.wait_for_surface
whats_on = DESKTOP.whats_on
window_titled = DESKTOP.window_titled
pictures = DESKTOP.pictures
