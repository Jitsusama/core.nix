# Every Mac: home-manager inside the system, nixpkgs' settings, Touch ID for
# sudo and zsh's macOS side, with the base role in every account.
{ darwinModules, homeModules }:
{
  imports = [
    darwinModules.home-manager
    darwinModules.nixpkgs
    darwinModules.sudo
    darwinModules.zsh
  ];
  home-manager.sharedModules = [ homeModules.base ];
}
