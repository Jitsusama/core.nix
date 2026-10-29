# The stick every guide VM test boots: core's stick system, built as a test
# node so the driver can reach its shell, and never started itself, since a
# machine boots its image from USB. It carries set-pin for the stand-in
# YubiKeys (canokeys.nix) whether a test plugs them in or not, so every test
# that takes this node boots one image, built once for them all rather than
# once a test.
{ self, pkgs }:
{ lib, ... }:
{
  imports = [ self.nixosModules.guide-stick ];
  virtualisation = {
    fileSystems = lib.mkVMOverride { };
    directBoot.enable = false;
    mountHostNixStore = false;
    useEFIBoot = true;
  };
  environment.systemPackages = [ (import ./canokeys.nix { inherit pkgs; }).setPin ];
}
