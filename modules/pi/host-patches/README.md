# The Host's Patches

The patches hosted-pi.nix applies to pi as a host runs it, exactly and
never fuzzed, each saying at its top what it changes and why. Every pi an
account starts by hand stays unpatched. Each has a check in ../checks.nix
that runs the patched pi, under a name beginning `pi-hosted-`.
