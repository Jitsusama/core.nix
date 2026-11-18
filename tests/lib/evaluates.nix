# A check that passes when a machine evaluates and no module warns, for
# core's checks and for any repository built on core, as
# `core.lib.evaluates pkgs name machine`. Writing out the path of its
# derivation forces the whole configuration while building none of it. A
# warning is a setting doing nothing, or soon to, printed in every
# evaluation's log until someone reads it. It reads the machine's
# `warnings`, which modules give; a trace that Nix prints as it evaluates,
# such as a renamed package's, never reaches them, and only Nix's
# abort-on-warn setting stops on it. Failing, it says the check's name and
# each warning, as text alone, so a warning naming a package builds none.
{
  lib,
  writeText,
  runCommandLocal,
}:
name: machine:
let
  warned = machine.config.warnings;
  said = builtins.unsafeDiscardStringContext "${name} warns: ${lib.concatStringsSep "; " warned}";
in
if warned != [ ] then
  runCommandLocal name { } ''
    echo ${lib.escapeShellArg said}
    exit 1
  ''
else
  writeText name (builtins.unsafeDiscardStringContext machine.config.system.build.toplevel.drvPath)
