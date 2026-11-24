# A Mac Joel writes code on: the YubiKey to get his keys back, with the
# workstation role in every account, and commits signed with keys in its
# Secure Enclave. The pi hosts its account names outlive a log-out. Builds
# churn its disk, so a guard in the machine and in every account takes what
# regenerates before the disk fills. Its Nix signs what it builds, so another
# machine can take that by key. The pi sessions a restart ends come back by
# themselves once it's up again.
{ darwinModules, homeModules }:
{
  imports = [
    darwinModules.base
    darwinModules.disk-guard
    darwinModules.mycelium
    darwinModules.nix-daemon
    darwinModules.pi
    darwinModules.yubikey
  ];
  home-manager.sharedModules = [
    homeModules.workstation
    homeModules.disk-guard
    homeModules.name-lookups
    homeModules.secure-enclave-signing
    homeModules.sessions-come-back
  ];
}
