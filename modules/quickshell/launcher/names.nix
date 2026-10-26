# Every character's name, for the launcher, from the copy of Unicode's
# names Python carries: its version, the runs named by their code point,
# then a line for each other character. names.py says how.
{ lib, pkgs }:
pkgs.runCommandLocal "character-names" { } ''
  ${lib.getExe pkgs.python3} ${./names.py} > $out
''
