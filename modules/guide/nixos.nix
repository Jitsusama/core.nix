# The guide on a machine it installed, for the steps that finish it there
# and the repairs after: the program, built with the machine's own systemd
# so its bootctl and cryptenroll are the ones the machine boots with, and
# what it reads about the machine (src/installed/mod.rs's Facts).
{ config, pkgs, ... }:
{
  # What the guide does as root goes through run0, which systemd lets a
  # person do only when polkit says they may.
  security.polkit.enable = true;

  environment.systemPackages = [
    (pkgs.callPackage ./package.nix { systemd = config.systemd.package; })
  ];
  environment.etc."guide/machine.json".text = builtins.toJSON {
    disk_name = config.jitsusama.disk.name;
  };
}
