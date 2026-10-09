# A NixOS machine Joel writes code on, with the workstation role in every
# account, and Rust linking with mold, which only Linux needs.
{ nixosModules, homeModules }:
{
  imports = [
    nixosModules.base
    nixosModules.memory
    nixosModules.perf
  ];
  home-manager.sharedModules = [
    homeModules.workstation
    homeModules.cargo
  ];
}
