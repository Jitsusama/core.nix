# A machine Joel writes code on: Neovim and coding agents, the broker that
# hands their sandboxes GitHub tokens an hour each, mycelium to carry their
# mesh to his other machines, and the ErgoDox he plugs into each one.
{ homeModules }:
{
  imports = [
    homeModules.base
    homeModules.neovim
    homeModules.claude
    homeModules.opencode
    homeModules.pi
    homeModules.github-broker
    homeModules.mycelium
    homeModules.ergodox
  ];
}
