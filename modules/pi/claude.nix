# pi-claude, pi on Joel's own Claude plan: claude.sh says how. The provider
# (claude-provider.nix) is loaded only by this command, so no other pi session
# lists models it can't reach. pi itself is whichever is on the PATH, so this
# follows the pi Joel runs everywhere else. On a machine whose only Claude
# login is the plan's (ownLogin), it reads Claude Code's own login rather than
# one kept apart. It starts on `model` unless given one.
{
  pkgs,
  claude,
  model ? "claude-sonnet-5",
  ownLogin ? false,
}:
pkgs.writeShellApplication {
  name = "pi-claude";
  runtimeInputs = [ pkgs.coreutils ];
  runtimeEnv = {
    CLAUDE = pkgs.lib.getExe claude;
    PROVIDER = import ./claude-provider.nix { inherit pkgs; };
    OWN_LOGIN = if ownLogin then "1" else "";
    PLAN_MODEL = model;
  };
  text = builtins.readFile ./claude.sh;
}
