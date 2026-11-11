# systemd-run as agents meet it, keeping their units in their own slice,
# as systemd-run.sh says why. The real one and the cgroup file are given,
# so the check can build it in front of a stand-in.
{
  lib,
  systemd,
  writeShellApplication,
  real ? "${systemd}/bin/systemd-run",
  cgroup ? "/proc/self/cgroup",
}:
lib.hiPrio (writeShellApplication {
  name = "systemd-run";
  text = builtins.replaceStrings [ "@real@" "@cgroup@" ] [ real cgroup ] (
    builtins.readFile ./systemd-run.sh
  );
})
