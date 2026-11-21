# call-under-load makes a test call shaped as Meet's each hour, on Chrome's
# fake camera, while the machine carries whatever it carries, and writes
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
    Unit = {
      Description = "Measures Joel's calls and apps under load";
      # It reads the meetings from the google daemon, so with no daemon (an
      # account signed out of Google, a VM test) it waits rather than failing;
      # cadence still says when the hour's figure goes stale.
      ConditionPathExists = "%t/daemons/google.sock";
    };
    Service = {
      Type = "oneshot";
      ExecStart = "${lib.getExe package} run";
      # In the apps' slice, as Joel's own calls are, so the shaper classes
      # its traffic as a call's and the apps' share of the CPU is what it
      # gets: the figures are what a call of his would see.
      Slice = "app.slice";
      # A call runs 150 s and Chrome takes up to a minute to start under
      # load; one stuck past this is ended, its Chromes with it.
      RuntimeMaxSec = "10min";
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
