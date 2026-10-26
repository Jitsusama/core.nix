# pi-ask, pi for the launcher's Ask row: ask.sh says how. The plan, the model,
# the extensions and the folders put ahead on the PATH are the machine's (jitsusama.pi.ask in home.nix), so the
# same row answers on a work machine through its proxy and on a personal one
# through Joel's own Claude plan. pi itself is whichever is on the PATH, as
# for pi-claude. Each question starts with a digest of what's true on the
# machine now (ask-digest.ts), naming the person the machine names.
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
    ASK_PATH = lib.concatLines ask.path;
    PI_CLAUDE = lib.getExe piClaude;
    DIGEST = "${./ask-digest.ts}";
    ASK_PERSON = if ask.person == null then "" else ask.person;
  };
  text = builtins.readFile ./ask.sh;
}
