# Every Mac: home-manager inside the system, nixpkgs' settings, an sshd that
# lets a gone caller go, Touch ID for sudo and zsh's macOS side, with the
# base role in every account.
{ darwinModules, homeModules }:
{
  imports = [
    darwinModules.home-manager
    darwinModules.nixpkgs
    darwinModules.openssh
    darwinModules.sudo
    darwinModules.zsh
  ];
  home-manager.sharedModules = [ homeModules.base ];
}
