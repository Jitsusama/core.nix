# zsh as every account's login shell, as it is on a Mac. home-manager
# configures the shell in each account but leaves the login shell to the
# system, so without this an account on NixOS logs in to bash and never reads
# its zsh settings.
{ pkgs, ... }:
{
  imports = [ ./system.nix ];

  users.defaultUserShell = pkgs.zsh;

  # Every shell would run dircolors to set colours vivid's, written when the
  # account is built, then replace (modules/vivid).
  programs.zsh.enableLsColors = false;
}
