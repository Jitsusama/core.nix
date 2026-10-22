# The marks every surface draws from, looked up by meaning, each with a
# stand-in a console font can draw. The set is pi's, which chose marks the
# monospace font has so columns stay aligned. The theme module's glyphs
# option starts from it (home.nix), and the guide draws from it too, since
# it runs where no account's theme is (../guide/theme.nix).
{ lib }:
let
  # A mark from the icon font by its code point, since an editor may not
  # show one.
  nerd = import ./nerd.nix { inherit lib; };
in
{
  cursor = {
    glyph = "▸";
    console = ">";
    means = "the current row or choice";
  };
  separator = {
    glyph = "·";
    console = ".";
    means = "a gap between two fields on one line";
  };
  complete = {
    glyph = "◆";
    console = "*";
    means = "done, approved, passed";
  };
  pending = {
    glyph = "◇";
    console = "o";
    means = "waiting, unanswered, not started";
  };
  active = {
    glyph = "◈";
    console = "@";
    means = "in progress, the mode in use";
  };
  failed = {
    glyph = "✕";
    console = "x";
    means = "failed, rejected, an error";
  };
  on = {
    glyph = "●";
    console = "*";
    means = "a mode that's on, a state worth seeing";
  };
  stopped = {
    glyph = "■";
    console = "#";
    means = "a mode or a run that's stopped";
  };
  queued = {
    glyph = "◦";
    console = "o";
    means = "accepted, nothing spent on it yet";
  };
  running = {
    glyph = "→";
    console = ">";
    means = "sent away and working";
  };
  done = {
    glyph = "✓";
    console = "v";
    means = "finished well";
  };
  cancelled = {
    glyph = "−";
    console = "-";
    means = "stopped by someone, neither passed nor failed";
  };
  rule = {
    glyph = "─";
    console = "-";
    means = "a line between sections";
  };
  light_rule = {
    glyph = "┄";
    console = "-";
    means = "a lighter line, within a section";
  };
  synced = {
    glyph = nerd "f021";
    console = "S";
    means = "input goes to every pane at once";
  };
  fullscreen = {
    glyph = nerd "f0b2";
    console = "F";
    means = "one thing fills the space";
  };
  floating = {
    glyph = nerd "f2d0";
    console = "W";
    means = "something floats above the layout";
  };
  awake = {
    glyph = nerd "f0176";
    console = "C";
    means = "the machine is kept from sleeping";
  };
  waiting = {
    glyph = nerd "f116b";
    console = "N";
    means = "notices came and went unseen";
  };
  quiet = {
    glyph = nerd "f00a0";
    console = "Q";
    means = "notices are held, not shown";
  };
  muted = {
    glyph = nerd "f036d";
    console = "M";
    means = "the microphone hears nothing";
  };
  call = {
    glyph = nerd "f03f2";
    console = "T";
    means = "a call is under way";
  };
  sharing = {
    glyph = nerd "f1483";
    console = "V";
    means = "another program sees the screen";
  };
  recording = {
    glyph = nerd "f0ec2";
    console = "R";
    means = "the screen is being recorded";
  };
  # A webcam, unlike the camera that stands for taking a picture, and its
  # console letter K, since C is awake's.
  camera = {
    glyph = nerd "f05a0";
    console = "K";
    means = "a program has a camera open";
  };
  playing = {
    glyph = nerd "f075a";
    console = "P";
    means = "music is playing";
  };
  capitals = {
    glyph = nerd "f0632";
    console = "A";
    means = "caps lock is on";
  };
  layer = {
    glyph = nerd "f0328";
    console = "L";
    means = "the keyboard is locked on another layer";
  };
  low = {
    glyph = nerd "f0083";
    console = "B";
    means = "the battery is running low";
  };
  disk = {
    glyph = nerd "f02ca";
    console = "D";
    means = "a disk is short of room, a writer was frozen, or nothing watches it";
  };
  spend = {
    glyph = nerd "f01c1";
    console = "$";
    means = "AI spend wants a look: a cap near or reached, a burn, or nothing watching it";
  };
  more = {
    glyph = "…";
    console = "~";
    means = "something cut short, or more hidden";
  };
  prompt = {
    glyph = "❯";
    console = ">";
    means = "where a shell's keys go next";
  };
  root = {
    glyph = "#";
    console = "#";
    means = "where a shell's keys go next, reaching everything";
  };
  elsewhere = {
    glyph = "@";
    console = "@";
    means = "on another machine, named after it";
  };
  branch = {
    glyph = nerd "f418";
    console = "b";
    means = "a git branch, named after it";
  };
  shell = {
    glyph = nerd "f313";
    console = "N";
    means = "a development shell is loaded";
  };
  unpushed = {
    glyph = nerd "f431";
    console = "^";
    means = "commits not yet pushed";
  };
  locked = {
    glyph = nerd "f023";
    console = "%";
    means = "can be read but not changed";
  };
  hidden = {
    glyph = "•";
    console = "*";
    means = "a character of a secret, hidden as it's typed";
  };
  caution = {
    glyph = nerd "f071";
    console = "!";
    means = "a warning: worth a look, though nothing failed";
  };
  note = {
    glyph = nerd "f05a";
    console = "i";
    means = "something to know, asking nothing";
  };
  suggestion = {
    glyph = nerd "f0eb";
    console = "?";
    means = "a way it could be better";
  };
  agent = {
    glyph = "π";
    console = "p";
    means = "an agent: a pi session, or one paired with this editor";
  };
  thinking = {
    glyph = nerd "f09d1";
    console = "t";
    means = "how hard an agent thinks, in its level's colour";
  };
}
