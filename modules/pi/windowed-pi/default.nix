# pi as a windowed session runs it, built from pi's own source with every
# windowed patch here, applied exactly, never fuzzed, so a pi that moved
# under them fails a check rather than in a window. A repository built on
# core builds it through the flake's lib.windowedPi; a host's pi is
# hosted-pi.nix's, which takes none of these.
{ pkgs }:
let
  windowedPatches = [
    ./pi-windowed-reads-left-out-entries-light.patch
  ];
in
pkgs.pi-coding-agent.overrideAttrs (old: {
  patches = (old.patches or [ ]) ++ windowedPatches;
  patchFlags = [
    "-p1"
    "--fuzz=0"
  ];
})
