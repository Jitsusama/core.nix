# A Mac Joel writes code on: the YubiKey to get his keys back, with the
# workstation role in every account.
{ darwinModules, homeModules }:
{
  imports = [
    darwinModules.base
    darwinModules.yubikey
  ];
  home-manager.sharedModules = [ homeModules.workstation ];
}
