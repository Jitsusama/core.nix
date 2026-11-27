# pi as a Windowed Session Runs It

`default.nix` builds pi with the patches beside it, applied exactly and
never fuzzed, each saying at its top what it changes and why. A host's pi
takes none of them; its own are in ../host-patches. Each has a check in
`checks.nix` that runs the patched pi, under a name beginning
`pi-windowed-`, which pi's own checks.nix gathers, and the flake gives
the build to a repository built on core as `lib.windowedPi`.
