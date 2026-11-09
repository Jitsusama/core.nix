# The stick every guide VM test boots: core's stick system, built as a test
# node so the driver can reach its shell, and never started itself, since a
# machine boots its image from USB. It carries set-pin for the stand-in
# YubiKeys (canokeys.nix) whether a test plugs them in or not, so every test
# that takes this node boots one image, built once for them all rather than
# once a test. It carries a stand-in for Apple's restore image too, a zip of
# a few bytes under Apple's name, so the partition a machine repository's
# stick carries is laid out and written as Apple's would be.
{ self, pkgs }:
{ lib, ... }:
{
  imports = [ self.nixosModules.guide-stick ];
  jitsusama.stick.appleRestores = [ (import ./apple-restore.nix { inherit pkgs; }) ];
  virtualisation = {
    fileSystems = lib.mkVMOverride { };
    directBoot.enable = false;
    mountHostNixStore = false;
    useEFIBoot = true;
  };
  environment.systemPackages = [ (import ./canokeys.nix { inherit pkgs; }).setPin ];
}
