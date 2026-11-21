# pi-keys samples, five times a day, what a key typed into the pi window
# Joel is at waits on (src/main.rs says how), and keeps a line a sample in
# $XDG_STATE_HOME/pi-keys/<UTC day>.jsonl, so a day's typing in pi can be
# read beside the others without anyone measuring it by hand. Each line
# says how busy the machine was and which pi it read, so a rise can be
# pinned on the load or on an install.
{ lib, pkgs, ... }:
let
  package = pkgs.callPackage ./package.nix { };
  calls = pkgs.callPackage ../calls/package.nix { };
in
{
  home.packages = [ package ];

  systemd.user.services.pi-keys = {
    Unit = {
      Description = "Samples what a key typed into pi waits on";
      After = [ "graphical-session.target" ];
      # A sample runs as a oneshot for up to seventy minutes, and a
      # switch that restarted one would wait on its start until
      # home-manager gave up and the whole switch went back, as one did
      # at 19:00 on 20 November. The sample running finishes on the old
      # program, and the timer's next starts the new.
      X-SwitchMethod = "keep-old";
    };
    Service = {
      Type = "oneshot";
      # A sample stops the moment a call starts, so it never reads a
      # thread while Joel is on one.
      ExecStart = "${lib.getExe package} sample --call-check ${lib.getExe calls}";
      # Beside the compositor and sound, as lag-watch is, so it isn't
      # starved while it measures what starves them; it reads six small
      # files every 2 ms for ten minutes, about half a percent of a core.
      Slice = "session.slice";
    };
  };

  # Through the day, so the hours Joel types most are each sampled; a
  # sample waits up to an hour for him to be at a pi window. The evening
  # gets one too: in the eight days from 8 November, 370 of the 1,595 messages
  # he typed into pi came between 20:00 and midnight, after the 19:00 sample.
  systemd.user.timers.pi-keys = {
    Unit.Description = "Samples pi's keys five times a day";
    Timer.OnCalendar = [
      "*-*-* 10:00"
      "*-*-* 13:00"
      "*-*-* 16:00"
      "*-*-* 19:00"
      "*-*-* 22:00"
    ];
    Install.WantedBy = [ "timers.target" ];
  };
}
