# pi as a Windowed Session Runs It

`default.nix` builds pi with the patches beside it, applied exactly and
never fuzzed, each saying at its top what it changes and why. It takes
three of the host's too, from ../host-patches: the loader's, which keep a
keyed package's modules once, since a windowed pi loads its extensions the
way a host does. A host's pi takes none of the patches beside it. Each
patch a windowed pi takes has a check in `checks.nix` that runs the
patched pi, under a name beginning `pi-windowed-`, which pi's own
checks.nix gathers, and the flake gives the build to a repository built on
core as `lib.windowedPi`.
