# Every time zone's changes, for the launcher, from tzdata: a line of JSON
# each, [zone, names, changes], from 2024 to the end of 2045, past which a
# zone keeps its last. zones.py says how.
{ lib, pkgs }:
pkgs.runCommandLocal "time-zones" { } ''
  ${lib.getExe pkgs.python3} ${./zones.py} ${pkgs.tzdata}/share/zoneinfo 2024 2045 > $out
''
