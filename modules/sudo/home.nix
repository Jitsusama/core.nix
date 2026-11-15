# sudo at the account's terminal on NixOS takes a YubiKey's touch at the
# unlocked desk, as run0 does, by going through run0 for its plain forms
# (by-touch.sh): `sudo nixos-rebuild switch` asks polkit, whose prompt
# names the command, takes the touch once the person is in, and asks for
# the password over ssh, at a text console, with the screen locked or with
# no key in (modules/yubikey). Only the account's interactive zsh changes,
# so scripts, and agents, which run no interactive shell, keep sudo
# itself, and sudo's own forms, such as another user, still ask for the
# password. A Mac's sudo takes Touch ID itself (darwin.nix).
{ lib, pkgs, ... }:
let
  byTouch = pkgs.callPackage ./by-touch.nix {
    run0 = "/run/current-system/sw/bin/run0";
    sudo = "/run/wrappers/bin/sudo";
  };
in
lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
  programs.zsh.shellAliases.sudo = lib.getExe byTouch;
}
