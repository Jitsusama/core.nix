# A machine Joel writes code on: Neovim, zellij and coding agents, and the
# ErgoDox he plugs into each one.
{ homeModules }:
{
  imports = [
    homeModules.base
    homeModules.neovim
    homeModules.zellij
    homeModules.claude
    homeModules.opencode
    homeModules.pi
    homeModules.ergodox
  ];
}
