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
      # A sample is a oneshot that waits up to an hour for a pi window and
      # then samples ten minutes at one, so it can run well past an hour.
      # A switch that restarted one waited on its start until sd-switch
      # gave up and the whole switch went back, as one at 19:02 on 20
      # November did at 19:11, after the 19:00 sample began. The sample
      # running finishes on the old program, and the timer's next starts
      # the new.
      X-SwitchMethod = "keep-old";
    };
    Service = {
      Type = "oneshot";
      # A switch no longer ends a hung sample, so this does, before the
      # timer's next, three hours on, would find it still running.
      TimeoutStartSec = "2h30min";
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
