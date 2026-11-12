# A machine Joel writes code on: Neovim and coding agents, the broker that
# hands their sandboxes GitHub tokens an hour each, the one that makes
# their fenced commands' scopes, mycelium to carry their
# mesh to his other machines, the to-do daemon that clears what the agents
# ask of him by itself, and the ErgoDox he plugs into each one.
{ homeModules }:
{
  imports = [
    homeModules.base
    homeModules.neovim
    homeModules.claude
    homeModules.opencode
    homeModules.pi
    homeModules.github-broker
    homeModules.jobs-broker
    homeModules.gcloud-broker
    homeModules.mycelium
    homeModules.to-do
    homeModules.ergodox
  ];
}
