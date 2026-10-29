# The checks of bluetooth, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves. A virtual machine
# has no Bluetooth controller to wake it, so what's checked is the rule the
# wake needs, as udev reads it.
{
  bareNixos,
  lib,
  onLinux,
  pkgs,
  self,
  ...
}:
onLinux {
  # A machine with Bluetooth carries the rule that lets its Bluetooth
  # controller wake it, on USB or on PCI, and udev reads that rule without a
  # complaint.
  a-bluetooth-keyboard-may-wake-it =
    let
      inherit ((bareNixos [ self.nixosModules.bluetooth ]).config.services.udev) packages;
      wakes = lib.findFirst (
        package: lib.hasInfix "bluetooth-wakes" package.name
      ) (throw "no udev package gives the Bluetooth wake rule") packages;
      udev = {
        nativeBuildInputs = [ pkgs.systemd ];
      };
    in
    pkgs.runCommandLocal "a-bluetooth-keyboard-may-wake-it" udev ''
      rules=${wakes}/lib/udev/rules.d/70-bluetooth-wakes.rules
      udevadm verify --resolve-names=never "$rules"
      for match in 'Class}=="e0"' 'SubClass}=="01"' 'Protocol}=="01"' 'class}=="0x0d1100"' 'wakeup}="enabled"'; do
        grep -qF "$match" "$rules"
      done
      touch $out
    '';
}
