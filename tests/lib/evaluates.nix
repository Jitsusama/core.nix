# A check that passes when a machine evaluates and warns of nothing, for
# core's checks and for any repository built on core, as
# `core.lib.evaluates pkgs name machine`. Writing out the path of its
# derivation forces the whole configuration while building none of it. A
# warning is a setting doing nothing, or soon to, printed in every
# evaluation's log until someone reads it.
{
  lib,
  writeText,
  runCommandLocal,
}:
name: machine:
let
  warned = machine.config.warnings;
in
if warned != [ ] then
  runCommandLocal name { } ''
    echo ${lib.escapeShellArg "${name} warns: ${lib.concatStringsSep "; " warned}"}
    exit 1
  ''
else
  writeText name (builtins.unsafeDiscardStringContext machine.config.system.build.toplevel.drvPath)
