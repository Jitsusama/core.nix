# A Mac Joel writes code on: the YubiKey to get his keys back, with the
# workstation role in every account, and commits signed with keys in its
# Secure Enclave. Builds churn its disk, so a guard in the machine and in
# every account takes what regenerates before the disk fills.
{ darwinModules, homeModules }:
{
  imports = [
    darwinModules.base
    darwinModules.disk-guard
    darwinModules.yubikey
  ];
  home-manager.sharedModules = [
    homeModules.workstation
    homeModules.disk-guard
    homeModules.secure-enclave-signing
  ];
}
