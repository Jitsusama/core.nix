# pi

pi, the coding agent, as a machine runs it: `home.nix` for the account,
with its theme, keys, settings and fence; `host.nix` and `darwin.nix` for
the shared hosts each realm runs, built by `hosted-pi.nix` with the patches
in `host-patches`; the launcher's `ask.nix` and Claude's `claude.nix`, each
beside its program; and `checks.nix`, which gathers every check of these.
The doors, the agent proxy, the fence's programs, the spawner, the timers,
the proof a realm's node takes its growth and the checks' own programs each
have a folder of their own.

Many entries because each module here sits beside the program it builds and
the modules beside it import, by name: a command's Nix, its shell and its
TypeScript, or Claude's provider and its patches, are found by opening the
module that names them, which a folder for each would only put one step
further away.
