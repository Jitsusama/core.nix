# swayidle locks the screen when Joel steps away, before the laptop sleeps and
# whenever logind asks (loginctl lock-session), through Quickshell's lock
# screen. A while after locking, niri turns the screens off; any key or touch
# turns them back on.
#
# Two swayidles do it. One hears logind: it locks before sleep and when
# logind asks, unlocks when logind says to, and tells logind the session is
# idle. The other, swayidle-away (away.sh), asks logind nothing and does
# what happens as Joel steps away: it locks the screen, turns the screens
# off and keeps the mark a followed switch waits on. A swayidle that hears
# logind drops every timeout while anything holds logind's lock against
# going idle, and mycelium holds it whenever it holds work, so with one
# swayidle the screen never locked while Joel was away (2026-11-10). Keep
# Awake and a call's transcripts hold that lock to keep the screen on and
# unlocked, so swayidle-away asks screen-kept before locking or turning the
# screens off; Wayland's own inhibitor, held by a video or a call in
# Chrome, it hears through niri.
#
# Asking for the lock returns before it's drawn, and niri shows the desktop
# until every screen has its lock, so a machine let sleep at once could wake
# showing the desktop. The lock command returns once Quickshell says niri has
# drawn it (Lock.qml), and swayidle holds the sleep until it does, for up to
# three seconds, well inside logind's five. Then the screens go off
# (lock-for-sleep.sh), so a hibernation goes down dark, and they come back
# on once the machine has woken, to the lock after a suspend.
#
# When logind says to unlock, as after a resume from hibernation, where the
# disk's prompt has already asked for the password, the lock lets go without
# asking again.
#
# As the screens go off, swayidle tells logind the session is idle, which on
# battery lets idle sleep have the laptop (../idle-sleep/nixos.nix); any key
# or touch, an unlock or a wake says it isn't. While anything holds logind's
# lock against going idle, it doesn't say so, as mycelium and Keep Awake want.
#
# The screens go off and on through the system's niri, the one with core's patches,
# so the machine doesn't carry a second niri just to ask.
{
  lib,
  pkgs,
  osConfig ? { },
  ...
}:
let
  niri = osConfig.programs.niri.package or pkgs.niri;
  lock = pkgs.writeShellApplication {
    name = "lock-screen";
    runtimeInputs = [
      pkgs.coreutils
      pkgs.inotify-tools
      pkgs.quickshell
    ];
    text = builtins.readFile ./lock-screen.sh;
  };
  lockForSleep = pkgs.writeShellApplication {
    name = "lock-for-sleep";
    runtimeInputs = [
      lock
      niri
    ];
    text = builtins.readFile ./lock-for-sleep.sh;
  };
  screenKept = pkgs.writeShellApplication {
    name = "screen-kept";
    runtimeInputs = [
      pkgs.jq
      pkgs.systemd
    ];
    text = builtins.readFile ./screen-kept.sh;
  };
  # swayidle runs each command through sh, with this path.
  away = pkgs.writeShellApplication {
    name = "swayidle-away";
    runtimeInputs = [
      pkgs.bash
      pkgs.coreutils
      pkgs.swayidle
      lock
      niri
      screenKept
    ];
    text = builtins.readFile ./away.sh;
  };
  screensOn = "${lib.getExe niri} msg action power-on-monitors";
  # Seconds from the last key or touch to each step of stepping away: no
  # one marked at the machine, the lock, and the screens going off.
  unmarked = 120;
  # The mark goes with the session. A switch restarts swayidle-away while
  # the session carries on, and a restart is no key or touch, so then the
  # mark stays as it was.
  sessionEnded = pkgs.writeShellScript "swayidle-away-session-ended" ''
    ${lib.getExe' pkgs.systemd "systemctl"} --user is-active --quiet graphical-session.target ||
      ${lib.getExe' pkgs.coreutils "rm"} -f "$1"
  '';
  locked = 300;
  screensOff = 600;
  # The mark that someone is at the machine.
  here = "%t/someone-at-the-machine";
in
{
  home.packages = [
    lock
    lockForSleep
    screenKept
    away
  ];

  services.swayidle = {
    enable = true;
    # -w holds each sleep until its lock is drawn, as Home Manager's own
    # default does.
    extraArgs = [
      "-w"
      "idlehint"
      (toString screensOff)
    ];
    events = {
      before-sleep = lib.getExe lockForSleep;
      after-resume = screensOn;
      lock = "${lib.getExe lock} 'logind asked'";
      unlock = "${lib.getExe' pkgs.quickshell "qs"} ipc call lock release";
    };
  };

  # A followed switch waits while this marks someone touched a key or the
  # pointer in the last two minutes (../update-to-main/nixos.nix), since a
  # switch restarts the shell under their hands, and what moves Joel's
  # sessions waits for it to stay away. Only a key or a touch makes the
  # mark: the start used to touch it as well, so every switch read as Joel
  # at the machine for two minutes, stopping what waited for him to be
  # away (2026-11-11). A session ended has nobody.
  systemd.user.services.swayidle-away = {
    Unit = {
      Description = "Locks the screen and turns the screens off as Joel steps away";
      PartOf = [ "graphical-session.target" ];
      After = [ "graphical-session.target" ];
      ConditionEnvironment = "WAYLAND_DISPLAY";
    };
    Service = {
      ExecStart = "${lib.getExe away} ${toString unmarked} ${toString locked} ${toString screensOff}";
      ExecStopPost = "${sessionEnded} ${here}";
      Restart = "always";
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };
}
