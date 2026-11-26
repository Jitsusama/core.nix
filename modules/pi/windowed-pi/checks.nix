# The checks of pi as a windowed session runs it, which pi's checks.nix
# gathers beside its own, kept apart since they change with the patches
# beside them and the host's loader patches and tests a windowed pi takes.
# Each runs the patched pi.
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

  # A windowed pi keeps a keyed package's modules once, as a host does, with
  # the host's own tests: each folder its own extensions, a keyed package's
  # shared across folders and its modules once across its extensions. Each
  # runs twice, once as the unbundled loader runs and once as the bundle a
  # window starts does, which serves pi's own packages from inside itself.
  pi-windowed-shares-keyed-modules =
    let
      tests = lib.fileset.toSource {
        root = ../tests;
        fileset = lib.fileset.unions [
          ../tests/as-the-bundle-runs.mjs
          ../tests/hosted-keyed-packages.mjs
          ../tests/pi-hosted-keeps-each-folders-extensions.mjs
          ../tests/pi-hosted-keyed-late-imports.mjs
          ../tests/pi-hosted-shares-keyed-modules.mjs
          ../tests/pi-hosted-shares-only-keyed-packages.mjs
        ];
      };
      runs = [
        "pi-hosted-keeps-each-folders-extensions"
        "pi-hosted-shares-only-keyed-packages"
        "pi-hosted-shares-keyed-modules"
        "pi-hosted-keyed-late-imports"
      ];
      node = lib.getExe pkgs.nodejs;
      root = "${windowedPi}/lib/node_modules/pi-monorepo";
    in
    pkgs.runCommandLocal "pi-windowed-shares-keyed-modules" { } ''
      ${lib.concatMapStrings (test: ''
        HOME=$TMPDIR ${node} --no-warnings ${tests}/${test}.mjs ${root}
        HOME=$TMPDIR ${node} --no-warnings --import ${tests}/as-the-bundle-runs.mjs \
          ${tests}/${test}.mjs ${root}
      '') runs}
      touch $out
    '';
}
