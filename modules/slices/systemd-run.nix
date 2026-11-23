# systemd-run as agents meet it, keeping their units in their own slice,
# as systemd-run.sh says why. The real one and the cgroup file are given,
# so the check can build it in front of a stand-in. It reads a call's
# arguments by systemd-run-options.txt, the short letters that take a
# value and every long name, given it here.
{
  lib,
  systemd,
  writeShellApplication,
  real ? "${systemd}/bin/systemd-run",
  cgroup ? "/proc/self/cgroup",
}:
let
  lines = lib.splitString "\n" (builtins.readFile ./systemd-run-options.txt);
  options = map (lib.splitString " ") (
    lib.filter (line: line != "" && !lib.hasPrefix "#" line) lines
  );
  letter = option: builtins.elemAt option 0;
  long = option: builtins.elemAt option 1;
  valuedShort = lib.concatMapStrings letter (
    lib.filter (option: letter option != "-" && lib.hasSuffix "=" (long option)) options
  );
  longs = lib.concatMapStringsSep " " (option: ''"${long option}"'') options;
in
lib.hiPrio (writeShellApplication {
  name = "systemd-run";
  text =
    builtins.replaceStrings
      [ "@real@" "@cgroup@" "@valuedShort@" "@longs@" ]
      [ real cgroup valuedShort longs ]
      (builtins.readFile ./systemd-run.sh);
})
