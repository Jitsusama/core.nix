# A probe beside Joel's programs, at their weight, that wakes ten times a
# second and does nothing else (jobs-sorter/src/probe.rs), so probe.slice's
# cpu.pressure counts only the waiting the rest of the machine makes a
# program of his do. app.slice's own counts his programs making each other
# wait too: on 11 November nearly all of it was Slack's and Chrome's, and a
# watcher reading it stopped agents' work for his own Slack. A watcher
# deciding whether agents slow his programs reads the probe's instead.
{ config, pkgs, ... }:
let
  sorter = pkgs.callPackage ./jobs-sorter/package.nix { };
in
{
  systemd.user.slices.probe.sliceConfig.CPUWeight =
    config.systemd.user.slices.app.sliceConfig.CPUWeight;

  systemd.user.services.probe = {
    description = "Waits as a newly woken program would, for watchers to read";
    wantedBy = [ "default.target" ];
    serviceConfig = {
      Slice = "probe.slice";
      ExecStart = "${sorter}/bin/jobs-sorter probe";
      MemoryMax = "16M";
      Restart = "always";
      RestartSec = 1;
    };
  };
}
