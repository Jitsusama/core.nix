# A machine Joel writes code on: Neovim and coding agents, and the ErgoDox
# he plugs into each one.
{ homeModules }:
{
  imports = [
    homeModules.base
    homeModules.neovim
    homeModules.claude
    homeModules.opencode
    homeModules.pi
    homeModules.ergodox
  ];
}
