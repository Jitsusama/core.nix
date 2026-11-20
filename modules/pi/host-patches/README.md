# The Host's Patches

The patches hosted-pi.nix applies to pi as a host runs it, exactly and
never fuzzed, each saying at its top what it changes and why. Every pi an
account starts by hand stays unpatched. Each has a check in ../checks.nix
that runs the patched pi, under a name beginning `pi-hosted-`.

A runtime keeps a copy of each patch it applied under
`share/pi-host/patches`, by its name here, as hosted-pi.nix does. Cadence
tells a miss whose fix is a patch here fixed only once every running host
loads a runtime whose copy reads as the patch merged or a later version of
it on main, and keeps no copy of a patch the fix removed. So a runtime built
elsewhere keeps the same copies, taken from the very list it applies rather
than this whole folder, or its hosts' misses stay open or close falsely.
