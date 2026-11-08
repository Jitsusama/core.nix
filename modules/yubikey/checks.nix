# The checks of yubikey, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves. A virtual machine
# has no YubiKey to plug in, so what's checked is the rule as udev reads it,
# and that both the running system and its initrd carry it.
{
  holds,
  lib,
  macWorkstation,
  onLinux,
  pkgs,
  workstation,
  ...
}:
{
  # The YubiKey gets Joel's keys back on any machine he writes code on, so both
  # workstation roles bring its tools, a Mac as much as Linux.
  yubikey-on-every-workstation =
    let
      hasYkman =
        machine:
        lib.any (
          package: lib.getName package == "yubikey-manager"
        ) machine.config.environment.systemPackages;
    in
    holds "yubikey-on-every-workstation" "A workstation lacks the YubiKey's tools." (
      hasYkman workstation && hasYkman macWorkstation
    );
}
// onLinux {
  # A touched YubiKey types nothing, at the disk prompt or after: udev reads
  # the rule refusing its keyboard without a complaint, and a Linux
  # workstation carries it in the running system and in the initrd alike.
  a-yubikey-touch-types-nothing =
    let
      inherit (workstation.config) boot services;
      rules = ./types-nothing.rules;
      ruleText = builtins.readFile rules;
      inSystem = lib.any (
        package: lib.hasInfix "yubikey-types-nothing" package.name
      ) services.udev.packages;
      inInitrd = lib.hasInfix ruleText boot.initrd.services.udev.rules;
      udev = {
        nativeBuildInputs = [ pkgs.systemd ];
      };
    in
    assert lib.assertMsg inSystem "the running system lacks the YubiKey's keyboard rule";
    assert lib.assertMsg inInitrd "the initrd lacks the YubiKey's keyboard rule";
    pkgs.runCommandLocal "a-yubikey-touch-types-nothing" udev ''
      udevadm verify --resolve-names=never ${rules}
      for match in 'idVendor}=="1050"' 'bInterfaceClass}=="03"' 'bInterfaceProtocol}=="01"' \
        'authorized}="0"'; do
        grep -qF "$match" ${rules}
      done
      touch $out
    '';
}
