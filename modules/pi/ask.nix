# pi-ask, pi for the launcher's Ask row: ask.sh says how. The plan, the model
# and the extensions are the machine's (jitsusama.pi.ask in home.nix), so the
# same row answers on a work machine through its proxy and on a personal one
# through Joel's own Claude plan. pi itself is whichever is on the PATH, as
# for pi-claude.
{
  pkgs,
  lib,
  piClaude,
  ask,
}:
pkgs.writeShellApplication {
  name = "pi-ask";
  runtimeEnv = {
    PLAN = ask.plan;
    MODEL = ask.model;
    EXTENSIONS = lib.concatLines ask.extensions;
    PI_CLAUDE = lib.getExe piClaude;
  };
  text = builtins.readFile ./ask.sh;
}
