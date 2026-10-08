# A Mac Joel writes code on, with the workstation role in every account.
{ darwinModules, homeModules }:
{
  imports = [ darwinModules.base ];
  home-manager.sharedModules = [ homeModules.workstation ];
}
