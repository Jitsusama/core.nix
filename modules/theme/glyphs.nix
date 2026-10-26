# The marks every surface draws from, looked up by meaning, each with a
# stand-in a console font can draw. The set is pi's, which chose marks the
# monospace font has so columns stay aligned. The theme module's glyphs
# option starts from it (home.nix), and the guide draws from it too, since
# it runs where no account's theme is (../guide/theme.nix).
#
# A mark the margin's corner shows is drawn as well (`drawn`): an SVG path
# on a 12 by 12 grid, filled even-odd, every one touching the grid's top and
# bottom and centred across it. A font's glyph is drawn for text's size and
# goes to mush at half the gap, where these are drawn; the same drawing
# grows, whole, when What's on opens.
{ lib }:
let
  # A mark from the icon font by its code point, since an editor may not
  # show one.
  nerd = import ./nerd.nix { inherit lib; };
  # A drawing of several shapes, each its own part of the path.
  drawing = lib.concatStringsSep " ";
  # A struck mark is the mark with a gap cut along one side of the slash,
  # and the slash, as flat outlines worked out once. A slash filled even-odd
  # over the mark flips the mark under it instead, which at the margin's
  # size reads as neither.
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
  lidOpen = {
    glyph = nerd "f06ca";
    console = "c";
    means = "staying awake only while the lid is open";
  };
  awake = {
    glyph = nerd "f0176";
    console = "C";
    means = "the machine is kept from sleeping";
    # A mug, the grid's full height on its own, since a saucer drawn under a
    # cup read as a line of its own beneath it.
    drawn = drawing [
      "M0 0H9V9A3 3 0 0 1 6 12H3A3 3 0 0 1 0 9Z"
      "M9 2H10A2 3 0 0 1 10 8H9V6.5H10A0.5 1.5 0 0 0 10 3.5H9Z"
    ];
  };
  waiting = {
    glyph = nerd "f116b";
    console = "N";
    means = "notices came and went unseen";
    # A bell, its clapper below.
    drawn = drawing [
      "M6 0C8.8 0 10 2.4 10 5V8L12 10H0L2 8V5C2 2.4 3.2 0 6 0Z"
      "M4.5 10.5H7.5A1.5 1.5 0 0 1 4.5 10.5Z"
    ];
  };
  # The Do Not Disturb key, as the moon a Mac keyboard prints on its Focus
  # key: the key's own print, where the bell is how things stand.
  quietKey = {
    glyph = nerd "f0594";
    console = "z";
    means = "the key that holds notices, or lets them show";
  };
  quiet = {
    # The bell struck through, since the bell asleep, with its small z,
    # reads as a plain bell at the margin's size, beside the notices'.
    glyph = nerd "f009b";
    console = "Q";
    means = "notices are held, not shown";
    # Waiting's bell, struck through.
    drawn = drawing [
      "M4.2 0.4 L3.7 0.7 L3.3 1.1 L2.95 1.55 L11.4 10 L12 10 L10 8 L10 5 L9.95 4.35"
      "L9.9 3.75 L9.75 3.1 L9.6 2.55 L9.35 2 L9.05 1.5 L8.7 1.1 L8.3 0.7 L7.8 0.4 L7.25 0.2"
      "L6.65 0.05 L6 0 L5.35 0.05 L4.75 0.2Z M0 10 L7.15 10 L2 4.85 L2 8Z M4.95 11.55"
      "L5.25 11.8 L5.6 11.95 L6 12 L6.4 11.95 L6.75 11.8 L7.05 11.55 L7.3 11.25 L7.45 10.9"
      "L7.5 10.5 L4.5 10.5 L4.55 10.9 L4.7 11.25Z M12 10.6 L1.4 0 L0 1.4 L10.6 12Z"
    ];
  };
  ringing = {
    glyph = nerd "f009f";
    console = "!";
    means = "letting notices show again";
  };
  dictating = {
    glyph = nerd "f05cb";
    console = "D";
    means = "dictation hears what's said, to type it";
  };
  muted = {
    glyph = nerd "f036d";
    console = "M";
    means = "the microphone hears nothing";
    # The microphone on its stand, struck through.
    drawn = drawing [
      "M4.75 0.35 L4.5 0.5 L4.25 0.75 L4 1 L3.85 1.25 L3.7 1.55 L3.55 2.1 L1.4 0 L0 1.4"
      "L10.6 12 L12 10.6 L9.6 8.2 L10 7.55 L10.25 7.05 L10.4 6.55 L10.45 6.05 L10.5 5.5"
      "L9 5.5 L8.9 6.3 L8.6 7 L8.5 7.1 L8.15 6.75 L8.4 6.15 L8.5 5.5 L8.5 2.15 L8.4 1.85"
      "L8.3 1.55 L8.15 1.25 L8 1 L7.75 0.75 L7.5 0.5 L7.25 0.35 L6.95 0.2 L6.65 0.1 L6.35 0"
      "L6 0 L5.65 0 L5.35 0.1 L5.05 0.2Z M2 7.55 L2.25 8 L2.55 8.4 L2.9 8.8 L3.35 9.1"
      "L3.75 9.4 L4.25 9.65 L4.75 9.8 L5.25 9.95 L5.25 10.5 L3 10.5 L3 12 L9 12 L9 11.85"
      "L7.65 10.5 L6.75 10.5 L6.75 9.95 L7.05 9.85 L5.6 8.45 L5.2 8.4 L4.5 8.1 L3.9 7.6"
      "L3.4 7 L3.1 6.3 L3.05 5.9 L2.65 5.5 L1.5 5.5 L1.55 6.05 L1.6 6.55 L1.75 7.05Z"
    ];
  };
  call = {
    glyph = nerd "f03f2";
    console = "T";
    means = "a call is under way";
    # A handset.
    drawn = drawing [
      "M0 2.5L2.5 0L5.5 3L4 4.5C4.8 6.3 5.7 7.2 7.5 8L9 6.5L12 9.5L9.5 12"
      "C4.5 11.5 0.5 7.5 0 2.5Z"
    ];
  };
  joinCall = {
    glyph = nerd "f0659";
    console = "+";
    means = "joining a call already under way";
  };
  hangUp = {
    glyph = nerd "f03f5";
    console = "_";
    means = "leaving a call";
  };
  openRoom = {
    glyph = nerd "f081c";
    console = "#";
    means = "opening a room others can join a call in";
  };
  sharing = {
    glyph = nerd "f1483";
    console = "V";
    means = "another program sees the screen";
    # A screen on its stand.
    drawn = drawing [
      "M0 0H12V8.5H0Z M1.5 1.5V7H10.5V1.5Z"
      "M5.25 8.5H6.75V10.5H5.25Z M3 10.5H9V12H3Z"
    ];
  };
  recording = {
    glyph = nerd "f0ec2";
    console = "R";
    means = "the screen is being recorded";
    # The dot every recorder lights.
    drawn = "M0 6A6 6 0 1 0 12 6A6 6 0 1 0 0 6Z";
  };
  # A webcam, unlike the camera that stands for taking a picture, and its
  # console letter K, since C is awake's.
  camera = {
    glyph = nerd "f05a0";
    console = "K";
    means = "a program has a camera open";
    # A camera, its lens cut out.
    drawn = drawing [
      "M0 3H3L4 0H8L9 3H12V12H0Z"
      "M3.5 7.5A2.5 2.5 0 1 0 8.5 7.5A2.5 2.5 0 1 0 3.5 7.5Z"
    ];
  };
  playing = {
    glyph = nerd "f075a";
    console = "P";
    means = "music is playing";
    drawn = "M1 0L11 6L1 12Z";
  };
  # What music, or anything else that runs on, is asked to do.
  pause = {
    glyph = nerd "f03e4";
    console = "=";
    means = "pausing what's playing";
  };
  # The play key, which pauses too, as keyboards print it.
  playPause = {
    glyph = nerd "f040e";
    console = "p";
    means = "playing what's paused, or pausing what's playing";
  };
  next = {
    glyph = nerd "f04ad";
    console = ">";
    means = "going on to the next track";
  };
  previous = {
    glyph = nerd "f04ae";
    console = "<";
    means = "going back to the track before";
  };
  resume = {
    glyph = nerd "f040a";
    console = "P";
    means = "starting something stopped again: a track, a watch or a guard";
  };
  ask = {
    glyph = nerd "f0369";
    console = "?";
    means = "asking for more: a cap raised, or an override";
  };
  retry = {
    glyph = nerd "f0450";
    console = "~";
    means = "checking something again, as a network that wants a sign-in";
  };
  capitals = {
    glyph = nerd "f0632";
    console = "A";
    means = "caps lock is on";
    # Caps lock's own key: the arrow up, on its bar.
    drawn = "M6 0L12 6H8.5V9H3.5V6H0Z M3.5 10.5H8.5V12H3.5Z";
  };
  layer = {
    glyph = nerd "f0328";
    console = "L";
    means = "the keyboard is locked on another layer";
    # One layer above another.
    drawn = "M6 0L12 3.5L6 7L0 3.5Z M0 8.5L1.75 7.5L6 10L10.25 7.5L12 8.5L6 12Z";
  };
  layerOff = {
    glyph = nerd "f0329";
    console = "l";
    means = "letting the keyboard back off its locked layer";
  };
  low = {
    glyph = nerd "f0083";
    console = "B";
    means = "the battery is running low";
    # A battery standing, a little left at its foot.
    drawn = "M4 0H8V1.5H10V12H2V1.5H4Z M3.5 3H8.5V8.5H3.5Z";
  };
  disk = {
    glyph = nerd "f02ca";
    console = "D";
    means = "a disk is short of room, a writer was frozen, or nothing watches it";
    # A store, as a cylinder with a band.
    drawn = drawing [
      "M0 2A6 2 0 0 1 12 2V10A6 2 0 0 1 0 10Z"
      "M0 5A6 2 0 0 0 12 5V6.5A6 2 0 0 1 0 6.5Z"
    ];
  };
  processor = {
    glyph = nerd "f061a";
    console = "P";
    means = "the processor: how busy it is, its cores, its clock and its heat";
  };
  memory = {
    glyph = nerd "f035b";
    console = "M";
    means = "the memory: what's used, what's waiting for it and what holds it";
  };
  heat = {
    glyph = nerd "f050f";
    console = "H";
    means = "how hot the machine's parts run, and how often the processor slowed to cool";
  };
  fan = {
    glyph = nerd "f0210";
    console = "F";
    means = "a fan: how fast it turns, and how hard it's driven";
  };
  lag = {
    glyph = nerd "f051f";
    console = "W";
    means = "the desktop or its apps have waited on the same thing a while";
    # An hourglass.
    drawn = "M1 0H11V1.5L7 6L11 10.5V12H1V10.5L5 6L1 1.5Z";
  };
  spend = {
    glyph = nerd "f01c1";
    console = "$";
    means = "AI spend wants a look: a cap near or reached, a burn, or nothing watching it";
    # A price tag.
    drawn = "M0 0H7L12 6L7 12H0Z M2 5H4V7H2Z";
  };
  # What a glance at the desktop reads each value by (the launcher's foot),
  # so a number says what it's of without a word.
  charge = {
    glyph = nerd "f0079";
    console = "+";
    means = "the battery's charge";
  };
  charging = {
    glyph = nerd "f0084";
    console = "=";
    means = "the battery charging";
  };
  wireless = {
    glyph = nerd "f05a9";
    console = "w";
    means = "joined to a network through the air";
  };
  wired = {
    glyph = nerd "f0200";
    console = "e";
    means = "joined to a network by a cable";
  };
  traffic = {
    glyph = nerd "f04e1";
    console = "%";
    means = "what the network carries in and out, and how fast";
  };
  offline = {
    glyph = nerd "f05aa";
    console = "X";
    means = "joined to no network, or leaving the one joined";
  };
  volume = {
    glyph = nerd "f057e";
    console = "s";
    means = "how loud sound goes out";
  };
  # The sound keys, as keyboards print them.
  louder = {
    glyph = nerd "f075d";
    console = "+";
    means = "turning the sound up";
  };
  quieter = {
    glyph = nerd "f075e";
    console = "-";
    means = "turning the sound down";
  };
  silent = {
    glyph = nerd "f0581";
    console = "_";
    means = "sound goes nowhere: muted, or nothing to play it";
  };
  cost = {
    glyph = nerd "f0114";
    console = "c";
    means = "what AI use has cost";
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
  # The Super key, drawn as the system's own logo, as its keyboards' Super
  # keys print theirs.
  superKey = {
    glyph = nerd "f313";
    console = "S";
    means = "the Super key, in a chord";
  };
  # A swipe on the touchscreen, by the way it goes, in a chord.
  swipeUp = {
    glyph = nerd "f0740";
    console = "^";
    means = "a swipe up";
  };
  swipeDown = {
    glyph = nerd "f073d";
    console = "v";
    means = "a swipe down";
  };
  swipeLeft = {
    glyph = nerd "f073e";
    console = "<";
    means = "a swipe to the left";
  };
  swipeRight = {
    glyph = nerd "f073f";
    console = ">";
    means = "a swipe to the right";
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
  # A robot rather than π, which names the program running agents, not
  # what they are; AI's spend wears it too (Joel, 2026-10-24).
  agent = {
    glyph = nerd "f06a9";
    console = "p";
    means = "an agent, or what AI use has cost: a pi session, or one paired with this editor";
    # Something said that wants an answer.
    drawn = drawing [
      "M1 0H11A1 1 0 0 1 12 1V8A1 1 0 0 1 11 9H6.5L3 12V9H1A1 1 0 0 1 0 8V1"
      "A1 1 0 0 1 1 0Z"
    ];
  };
  # A person with a clock: something the agents can't go on without until
  # Joel has done it.
  waitsOnJoel = {
    glyph = nerd "f0b56";
    console = "W";
    means = "what the agents have waiting on Joel, for him to do";
  };
  thinking = {
    glyph = nerd "f09d1";
    console = "t";
    means = "how hard an agent thinks, in its level's colour";
  };
  # The launcher's things, each by what it is, as its vocabulary names
  # them (modules/quickshell/launcher/words.json).
  category = {
    glyph = nerd "f04fc";
    console = "#";
    means = "a category of the launcher's things, such as computer or ai";
  };
  command = {
    glyph = nerd "f018d";
    console = ">";
    means = "a command typed with what it takes, such as ip and an address";
  };
  windows = {
    glyph = nerd "f05b2";
    console = "w";
    means = "the windows open, or going to one";
  };
  workspace = {
    glyph = nerd "f11d9";
    console = "W";
    means = "a workspace, or moving a window to one";
  };
  below = {
    glyph = nerd "f03cc";
    console = "+";
    means = "a new window of a program";
  };
  copy = {
    glyph = nerd "f018f";
    console = "c";
    means = "copying something to the clipboard";
  };
  paste = {
    glyph = nerd "f0192";
    console = "v";
    means = "pasting into the window the launcher opened over";
  };
  close = {
    glyph = nerd "f05ad";
    console = "x";
    means = "closing a window";
  };
  forget = {
    glyph = nerd "f01fe";
    console = "-";
    means = "forgetting what's kept: the clipboard's copies, a network or a paired device";
  };
  clipboard = {
    glyph = nerd "f0a38";
    console = "C";
    means = "the clipboard and what it's kept";
  };
  notifications = {
    glyph = nerd "f009c";
    console = "N";
    means = "the notifications of the session, seen or missed";
  };
  emoji = {
    glyph = nerd "f01f5";
    console = ":";
    means = "emoji, to type";
  };
  shortcuts = {
    glyph = nerd "f097b";
    console = "K";
    means = "every layer's shortcuts";
  };
  # The shortcuts' layers and groups that no mark above means already.
  desktop = {
    glyph = nerd "f01c4";
    console = "d";
    means = "the desktop: its windows, columns, workspaces and screens";
  };
  touchscreen = {
    glyph = nerd "f0d76";
    console = "~";
    means = "the touchscreen's swipes";
  };
  columns = {
    glyph = nerd "f056d";
    console = "|";
    means = "a column of windows, or moving one";
  };
  launcher = {
    glyph = nerd "f0349";
    console = "/";
    means = "the launcher and the margin, finding what's there";
  };
  brightness = {
    glyph = nerd "f00e0";
    console = "*";
    means = "how bright the screen is, or the lights";
  };
  dimmer = {
    glyph = nerd "f00de";
    console = ".";
    means = "turning the screen's brightness down";
  };
  lamp = {
    glyph = nerd "f095f";
    console = "*";
    means = "the desk light, and its warmth";
  };
  session = {
    glyph = nerd "f033e";
    console = "L";
    means = "the session: locking it, or ending it";
  };
  keyboard = {
    glyph = nerd "f030c";
    console = "k";
    means = "a keyboard: its layout, layers, keys and lights, or one paired";
  };
  flash = {
    glyph = nerd "f0241";
    console = "!";
    means = "flashing a light to find what it's on, such as a keyboard";
  };
  # One screen rather than several, since most of what wears it is one
  # display, and the desktop's display shortcuts wore one already.
  display = {
    glyph = nerd "f0379";
    console = "D";
    means = "a display, or the displays";
  };
  bluetooth = {
    glyph = nerd "f00af";
    console = "B";
    means = "Bluetooth and what's paired by it";
  };
  bluetoothOff = {
    glyph = nerd "f00b2";
    console = "b";
    means = "Bluetooth turned off";
  };
  capture = {
    glyph = nerd "f0100";
    console = "P";
    means = "capturing the screen, a window, a region or the camera";
  };
  now = {
    glyph = nerd "f00f0";
    console = "!";
    means = "what wants Joel and what's on, now";
  };
  people = {
    glyph = nerd "f000e";
    console = "P";
    means = "calls, and the people to call or pair with";
  };
  saver = {
    glyph = nerd "f032a";
    console = "s";
    means = "the power saver profile";
  };
  balanced = {
    glyph = nerd "f05d1";
    console = "=";
    means = "the balanced power profile";
  };
  performance = {
    glyph = nerd "f04c5";
    console = "^";
    means = "the full performance power profile";
  };
  suspend = {
    glyph = nerd "f04b2";
    console = "z";
    means = "suspending the machine";
  };
  logout = {
    glyph = nerd "f0343";
    console = "<";
    means = "logging out";
  };
  reboot = {
    glyph = nerd "f0453";
    console = "r";
    means = "rebooting the machine";
  };
  poweroff = {
    glyph = nerd "f011";
    console = "o";
    means = "powering the machine off";
  };
  # What the launcher's own line and its empty list wear.
  greeting = {
    glyph = nerd "f0674";
    console = "*";
    means = "the launcher saying what it can do, before anything's typed";
  };
  nothingFound = {
    glyph = nerd "f02a0";
    console = "?";
    means = "nothing answers what was typed";
  };
  # The padlock, as locked's is, beside anything typed that stays hidden.
  secret = {
    glyph = nerd "f023";
    console = "%";
    means = "where a secret is typed: a password or a passphrase";
  };
  program = {
    glyph = nerd "f0614";
    console = "a";
    means = "a program, where it has no glyph of its own";
  };
  # The things Bluetooth pairs with, by the kind each says it is, and what's
  # done with them. Headphones and a speaker wear no Bluetooth badge, since
  # Sound shows the wired ones with the same marks and Bluetooth's own list
  # already says how they're joined.
  headphones = {
    glyph = nerd "f02cb";
    console = "h";
    means = "headphones";
  };
  headset = {
    glyph = nerd "f02ce";
    console = "h";
    means = "a headset: headphones with a microphone";
  };
  microphone = {
    glyph = nerd "f036c";
    console = "m";
    means = "a microphone, or anything else sound is heard through";
  };
  speaker = {
    glyph = nerd "f04c3";
    console = "s";
    means = "a speaker, or anything else sound plays through";
  };
  mouse = {
    glyph = nerd "f037d";
    console = "m";
    means = "a mouse or a tablet";
  };
  gamepad = {
    glyph = nerd "f0297";
    console = "g";
    means = "a game controller";
  };
  phone = {
    glyph = nerd "f011c";
    console = "t";
    means = "a phone";
  };
  computer = {
    glyph = nerd "f0322";
    console = "l";
    means = "a computer: the screen built into this one, or another paired";
  };
  connect = {
    glyph = nerd "f00b1";
    console = "+";
    means = "connecting to a device, or pairing with it";
  };
  # Bluetooth's own glyph struck through, as bluetoothOff is, since a
  # device let go is one Bluetooth no longer reaches.
  disconnect = {
    glyph = nerd "f00b2";
    console = "-";
    means = "letting a connected device go";
  };
  password = {
    glyph = nerd "f0306";
    console = "k";
    means = "what a network or a device asks to let it in: a password, an account, a passkey";
  };
  trust = {
    glyph = nerd "f0483";
    console = "t";
    means = "trusting a network: finding others on it, and being found";
  };
  accessPoint = {
    glyph = nerd "f0003";
    console = "a";
    means = "the access points a network is heard from";
  };
  tunnel = {
    glyph = nerd "f0582";
    console = "v";
    means = "a tunnel to another network, such as a VPN";
  };
  signIn = {
    glyph = nerd "f0342";
    console = ">";
    means = "signing in: to a network's page, or to an account";
  };
  # A network card, not the cable, since an address is a card's whether it
  # joins through the air or by wire.
  card = {
    glyph = nerd "f08ae";
    console = "n";
    means = "a network card, by its hardware address";
  };
  # A network's signal in four bars, then the same with a padlock for one
  # that asks for a password; the first of each is none heard.
  signal0 = {
    glyph = nerd "f092f";
    console = "0";
    means = "a network heard, its signal no bars";
  };
  signal1 = {
    glyph = nerd "f091f";
    console = "1";
    means = "a network heard, its signal one bar";
  };
  signal2 = {
    glyph = nerd "f0922";
    console = "2";
    means = "a network heard, its signal two bars";
  };
  signal3 = {
    glyph = nerd "f0925";
    console = "3";
    means = "a network heard, its signal three bars";
  };
  signal4 = {
    glyph = nerd "f0928";
    console = "4";
    means = "a network heard, its signal four bars";
  };
  lockedSignal0 = {
    glyph = nerd "f092c";
    console = "0";
    means = "a network with a password, its signal no bars";
  };
  lockedSignal1 = {
    glyph = nerd "f0921";
    console = "1";
    means = "a network with a password, its signal one bar";
  };
  lockedSignal2 = {
    glyph = nerd "f0924";
    console = "2";
    means = "a network with a password, its signal two bars";
  };
  lockedSignal3 = {
    glyph = nerd "f0927";
    console = "3";
    means = "a network with a password, its signal three bars";
  };
  lockedSignal4 = {
    glyph = nerd "f092a";
    console = "4";
    means = "a network with a password, its signal four bars";
  };
  # A display's settings, and turning one off; turning one on wears the
  # display's own glyph, as the radios' do.
  displayOff = {
    glyph = nerd "f0d90";
    console = "_";
    means = "a display turned off, or turning one off";
  };
  scale = {
    glyph = nerd "f06ed";
    console = "+";
    means = "how large a display draws things";
  };
  refresh = {
    glyph = nerd "f095b";
    console = "~";
    means = "how often a display draws itself again";
  };
  resolution = {
    glyph = nerd "f0a24";
    console = "#";
    means = "how many dots a display draws, across and down";
  };
  rotation = {
    glyph = nerd "f0475";
    console = "r";
    means = "which way up a display is turned";
  };
  newWorkspace = {
    glyph = nerd "f11da";
    console = "+";
    means = "a new workspace, made by moving a window to it";
  };
  # What a notice's program offers on it, and what's done with one.
  reply = {
    glyph = nerd "f045a";
    console = "<";
    means = "replying to a message from its notice";
  };
  offer = {
    glyph = nerd "f0ce0";
    console = ">";
    means = "doing what a notice's program offers on it";
  };
  read = {
    glyph = nerd "f01ef";
    console = "o";
    means = "marking a notice read";
  };
  unread = {
    glyph = nerd "f0b92";
    console = "*";
    means = "marking a notice unread";
  };
  dismiss = {
    glyph = nerd "f0a91";
    console = "x";
    means = "letting a notice go";
  };
  timed = {
    glyph = nerd "f1862";
    console = "t";
    means = "on for a while, then off by itself";
  };
  # What a copy is, so a link, a path or a number stands out from the text
  # around it; a copy of plain text wears the clipboard's.
  lines = {
    glyph = nerd "f09aa";
    console = "=";
    means = "text of several lines";
  };
  link = {
    glyph = nerd "f0337";
    console = "/";
    means = "a link to somewhere";
  };
  email = {
    glyph = nerd "f01f0";
    console = "@";
    means = "an email address";
  };
  path = {
    glyph = nerd "f0224";
    console = "/";
    means = "where a file or a folder is";
  };
  picture = {
    glyph = nerd "f02e9";
    console = "p";
    means = "a picture";
  };
  document = {
    glyph = nerd "f0219";
    console = "d";
    means = "a document, read as a page";
  };
  # What an answer in the field is about, by what was asked.
  number = {
    glyph = nerd "f03a0";
    console = "#";
    means = "a number, or a sum worked out";
  };
  swatch = {
    glyph = nerd "f14fb";
    console = "#";
    means = "a colour, shown in itself";
  };
  currency = {
    glyph = nerd "f0116";
    console = "$";
    means = "money, from one currency to another";
  };
  date = {
    glyph = nerd "f00ed";
    console = "d";
    means = "a date: today, or the day it is somewhere";
  };
  clock = {
    glyph = nerd "f0150";
    console = "o";
    means = "the time somewhere, and how far it's ahead of here or behind";
  };
  zone = {
    glyph = nerd "f01e7";
    console = "z";
    means = "a time zone";
  };
  # What's measured, for a measure from one unit to another.
  distance = {
    glyph = nerd "f046d";
    console = "|";
    means = "how long or how far: metres, miles";
  };
  mass = {
    glyph = nerd "f05d1";
    console = "m";
    means = "how heavy: grams, pounds";
  };
  capacity = {
    glyph = nerd "f01ab";
    console = "u";
    means = "how much something holds: litres, cups";
  };
  area = {
    glyph = nerd "f0fe6";
    console = "#";
    means = "how much ground or surface: square metres, acres";
  };
  speed = {
    glyph = nerd "f04c5";
    console = ">";
    means = "how fast: kilometres an hour, knots";
  };
  data = {
    glyph = nerd "f02ca";
    console = "b";
    means = "how much data: bytes, bits";
  };
  duration = {
    glyph = nerd "f051b";
    console = "t";
    means = "how long it takes: seconds, days";
  };
  energy = {
    glyph = nerd "f140b";
    console = "e";
    means = "how much energy: joules, calories";
  };
  pressure = {
    glyph = nerd "f029a";
    console = "p";
    means = "how hard something presses: pascals, bars";
  };
  angle = {
    glyph = nerd "f0937";
    console = "<";
    means = "how wide an angle: degrees, radians";
  };
  temperature = {
    glyph = nerd "f050f";
    console = "c";
    means = "how hot or cold: Celsius, Fahrenheit";
  };
  # What AI spend found and how it stands.
  urgent = {
    glyph = nerd "f0028";
    console = "!";
    means = "wanting Joel now, as a cap reached does";
  };
  trend = {
    glyph = nerd "f012a";
    console = "~";
    means = "something found in how a figure has moved";
  };
  fine = {
    glyph = nerd "f012c";
    console = "v";
    means = "nothing wants a look: all is well, or marked as expected";
  };
  report = {
    glyph = nerd "f0215";
    console = "=";
    means = "a report, to read in full";
  };
  # The weather, by day and by night where the sky looks different.
  sunny = {
    glyph = nerd "f0599";
    console = "o";
    means = "a clear sky by day";
  };
  clearNight = {
    glyph = nerd "f0594";
    console = "c";
    means = "a clear sky by night";
  };
  partlyCloudy = {
    glyph = nerd "f0595";
    console = "o";
    means = "some cloud by day";
  };
  partlyCloudyNight = {
    glyph = nerd "f0f31";
    console = "c";
    means = "some cloud by night";
  };
  cloudy = {
    glyph = nerd "f0590";
    console = "m";
    means = "a sky covered in cloud, or weather unknown";
  };
  fog = {
    glyph = nerd "f0591";
    console = "=";
    means = "fog";
  };
  showers = {
    glyph = nerd "f0f33";
    console = "'";
    means = "a little rain, or showers, by day";
  };
  rain = {
    glyph = nerd "f0597";
    console = "'";
    means = "rain, and the hours it's likely";
  };
  pouring = {
    glyph = nerd "f0596";
    console = "\"";
    means = "heavy rain";
  };
  sleet = {
    glyph = nerd "f067f";
    console = ",";
    means = "rain or drizzle that freezes";
  };
  snow = {
    glyph = nerd "f0598";
    console = "*";
    means = "snow";
  };
  heavySnow = {
    glyph = nerd "f0f36";
    console = "*";
    means = "heavy snow";
  };
  thunder = {
    glyph = nerd "f0593";
    console = "z";
    means = "thunderstorms";
  };
  hail = {
    glyph = nerd "f067e";
    console = "z";
    means = "thunderstorms with hail";
  };
  wind = {
    glyph = nerd "f059d";
    console = "~";
    means = "the wind";
  };
}
