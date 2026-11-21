# call-under-load makes a test call shaped as Meet's each hour, on
# Chromium's fake camera, while the machine carries whatever it carries, and writes
# what it did, with Chrome's, Slack's and the launcher's figures, for
# cadence to judge (src/main.rs says how). It never runs into a call of
# Joel's or within ten minutes of a meeting. `call-under-load` says the
# last hour's figures.
{ lib, pkgs, ... }:
let
  package = pkgs.callPackage ./package.nix { };
in
{
  home.packages = [ package ];

  systemd.user.services.call-under-load = {
    Unit.Description = "Measures Joel's calls and apps under load";
    Service = {
      Type = "oneshot";
      ExecStart = "${lib.getExe package} run";
      # It exits 69 when what it needs isn't there: the calendar unread (an
      # account signed out of Google, a VM test), Chrome missing, or no word
      # on whether Joel is in a call. That hour is skipped, not failed, and
      # cadence still says when the figure goes stale. The socket always
      # exists, since it's socket-activated, so it can't gate the unit.
      SuccessExitStatus = [ 69 ];
      # In the apps' slice, as Joel's own calls are, so the apps' share of
      # the CPU is what it gets: the figures are what a call of his would
      # see of the machine. Both ends are on the machine, so the line
      # carries none of it.
      Slice = "app.slice";
      # A call runs 150 s and Chrome takes up to a minute to start under
      # load; one stuck past this is ended, its Chromes with it. A oneshot's
      # run is its start, so the start's timeout bounds it.
      TimeoutStartSec = "10min";
    };
  };

  systemd.user.timers.call-under-load = {
    Unit.Description = "Measures Joel's calls and apps under load, each hour";
    Timer = {
      OnCalendar = "*:35";
      Persistent = false;
    };
    Install.WantedBy = [ "timers.target" ];
  };
}
