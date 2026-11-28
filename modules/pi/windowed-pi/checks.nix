# The checks of pi as a windowed session runs it, which pi's checks.nix
# gathers beside its own, kept apart since they change with the patches
# beside them alone. Each runs the patched pi.
{
  lib,
  pkgs,
  ...
}:
let
  # pi as a windowed session runs it, every windowed patch applied exactly.
  windowedPi = import ./. { inherit pkgs; };
in
{
  # A windowed pi reads light what its log's kept view leaves out, every
  # reader seeing what a whole read sees: a move to a left-out entry holds
  # its context whole, a later compaction lets go of what it leaves out
  # where pi knows the entry's place, a rewrite holds all whole first, and a
  # log pi migrates or with an unfinished last line is read whole.
  pi-windowed-reads-left-out-entries-light =
    pkgs.runCommandLocal "pi-windowed-reads-left-out-entries-light" { }
      ''
        HOME=$TMPDIR ${lib.getExe pkgs.nodejs} --no-warnings \
          ${../tests/pi-windowed-reads-left-out-entries-light.mjs} \
          ${windowedPi}/lib/node_modules/pi-monorepo
        touch $out
      '';
}
