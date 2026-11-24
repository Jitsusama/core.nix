# call-under-load makes a test call shaped as Meet's each hour, on
# Chromium's fake camera and microphone, while the machine carries whatever it carries, and writes
# what it did, with Chrome's, Slack's and the launcher's figures, for
# cadence to judge (src/main.rs says how). It never runs into a call of
# Joel's or within ten minutes of a meeting. `call-under-load` says the
# last hour's figures.
{
  lib,
  pkgs,
  # The machine's own settings, which Home Manager gives when it's part of
  # one, for its camera.
  osConfig ? { },
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  device = osConfig.jitsusama.camera.device or null;
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

  # Joel's own calls, watched as they happen: a test call says what a
  # call would see, but only his own say what his did. Each call's
  # sending waits join the history as it ends, and the next hour's
  # figures carry them for cadence.
  systemd.user.services.call-under-load-watch = {
    Unit.Description = "Watches Joel's own calls for sending threads kept from a CPU";
    Service = {
      ExecStart = "${lib.getExe package} watch";
      Restart = "on-failure";
      RestartSec = "30s";
      # In the apps' slice, weighted as any app, so under the heaviest
      # load, when a call is most worth reading, its asks of a-call-is-on
      # still answer in time ahead of agents' work. It reads a few files
      # each 5 s during a call and asks once each 30 s, well under 1% of
      # a core, so Joel's apps lose nothing to it.
      Slice = "app.slice";
    };
    Install.WantedBy = [ "default.target" ];
  };

  # The camera before each of Joel's meetings: three to six minutes out, with
  # no call on, it's read for five frames, none kept, and a camera giving no
  # picture is told to him then rather than seen by those he meets. The
  # built-in's first frame can take 40 s under memory pressure, so the read
  # is given two minutes.
  systemd.user.services.call-under-load-camera = {
    Unit.Description = "Checks Joel's camera gives a picture before each meeting";
    Service = {
      Type = "oneshot";
      ExecStart = lib.escapeShellArgs (
        [
          (lib.getExe package)
          "camera"
        ]
        ++ lib.optional (device != null) device
      );
      # The calendar unread, as in a VM test, leaves the minute unchecked.
      SuccessExitStatus = [ 69 ];
      # In the apps' slice, so its asks of a-call-is-on answer under load;
      # the camera's own work is the camera service's.
      Slice = "app.slice";
      TimeoutStartSec = "4min";
    };
  };

  systemd.user.timers.call-under-load-camera = {
    Unit.Description = "Checks Joel's camera before each meeting, each minute";
    Timer = {
      OnCalendar = "minutely";
      Persistent = false;
    };
    Install.WantedBy = [ "timers.target" ];
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
