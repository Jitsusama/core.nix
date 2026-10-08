# A machine Joel writes code on: Neovim, zellij and coding agents.
{ homeModules }:
{
  imports = [
    homeModules.base
    homeModules.neovim
    homeModules.zellij
    homeModules.claude
    homeModules.opencode
  ];
}
