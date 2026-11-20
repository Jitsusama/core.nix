# The Host's Patches

The patches hosted-pi.nix applies to pi as a host runs it, exactly and
never fuzzed, each saying at its top what it changes and why. Every pi an
account starts by hand stays unpatched. Each has a check in ../checks.nix
that runs the patched pi, under a name beginning `pi-hosted-`.

A runtime keeps a copy of each patch it applied under
`share/pi-host/patches`, by its name here, as hosted-pi.nix does. Cadence
tells a miss whose fix is a patch here fixed only once every running host
loads a runtime whose copy reads as the patch merged or a later version of
it, so a runtime built elsewhere keeps the same copies, or its hosts' misses
stay open.
