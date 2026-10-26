# Every character's name, for the launcher, from the copy of Unicode's
# names Python carries: its version, the runs named by their code point,
# then a line for each other character. character-names.py says how; its
# name is its own, as mypy reads every Python file as one module set.
{ lib, pkgs }:
pkgs.runCommandLocal "character-names" { } ''
  ${lib.getExe pkgs.python3} ${./character-names.py} > $out
''
