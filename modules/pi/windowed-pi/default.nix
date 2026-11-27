# pi as a windowed session runs it, built from pi's own source with every
# windowed patch here, applied exactly, never fuzzed, so a pi that moved
# under them fails a check rather than in a window. A repository built on
# core builds it through the flake's lib.windowedPi; a host's pi is
# hosted-pi.nix's, which takes none of these.
{ pkgs }:
let
  # A windowed pi loads its extensions through the same loader as a host,
  # so it takes the host's three patches that keep a keyed package's
  # modules once, in the host's order, from the host's own folder: one copy
  # of each, which both pis' checks hold to.
  loaderPatches = [
    ../host-patches/pi-host-extension-cache-per-folder.patch
    ../host-patches/pi-host-extension-cache-keyed-shared.patch
    ../host-patches/pi-host-extension-modules-keyed-shared.patch
  ];
  windowedPatches = loaderPatches ++ [
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
