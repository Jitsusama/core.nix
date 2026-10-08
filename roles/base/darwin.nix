# Every Mac: home-manager inside the system, nixpkgs' settings and zsh's macOS
# side, with the base role in every account.
{ darwinModules, homeModules }:
{
  imports = [
    darwinModules.home-manager
    darwinModules.nixpkgs
    darwinModules.zsh
  ];
  home-manager.sharedModules = [ homeModules.base ];
}
