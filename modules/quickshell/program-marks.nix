# Each program's own mark, by its desktop entry id in lower case, as
# ProgramGlyphs.js's table says it for every program the launcher shows,
# with the machine's own over it, written once as a file a daemon reads:
# a notice wears the mark its program's row does (../notices), and so
# does a program's row in Sound (../sound). The table is read line by
# line where it's written, never copied. `launcher` is the machine's
# launcher settings, or null on an account without the shell.
{
  lib,
  pkgs,
  launcher,
}:
let
  table = lib.pipe (builtins.readFile ./ProgramGlyphs.js) [
    (lib.splitString "\n")
    (map (builtins.match " *\"?([^\":]+)\"?: \"([^\"]*)\","))
    (builtins.filter (found: found != null))
    (map (found: lib.nameValuePair (builtins.elemAt found 0) (builtins.elemAt found 1)))
    lib.listToAttrs
  ];
  own = if launcher == null then { } else launcher.providers.applications.glyphs or { };
in
assert lib.assertMsg (
  table ? slack && table ? pi
) "ProgramGlyphs.js's table no longer reads as program-marks.nix reads it";
pkgs.writeText "program-marks.json" (
  builtins.toJSON (table // lib.mapAttrs' (id: lib.nameValuePair (lib.toLower id)) own)
)
