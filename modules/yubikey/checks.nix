# The checks of yubikey, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves. A virtual machine
# has no YubiKey to plug in, so what's checked is the rule as udev reads it,
# and that both the running system and its initrd carry it.
{
  bareNixos,
  holds,
  lib,
  macWorkstation,
  onLinux,
  pkgs,
  self,
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

  # A sign-in but polkit's that names pam_u2f, as a login, the greeter,
  # the lock or sudo would, stops the machine building, since a touch
  # there would let someone in with no password.
  only-polkit-may-take-a-yubikey-touch =
    let
      touchingLogin = bareNixos [
        self.nixosModules.workstation
        { security.pam.services.login.u2f.enable = true; }
      ];
      refused =
        machine:
        lib.any (
          check: !check.assertion && lib.hasInfix "YubiKey's touch" check.message
        ) machine.config.assertions;
    in
    holds "only-polkit-may-take-a-yubikey-touch"
      "A login takes a YubiKey's touch and the machine still builds, or a workstation with no key is refused."
      (refused touchingLogin && !refused workstation);

  # Once a key is named, polkit's sign-in takes its touch before the
  # password, from the root-owned file the system makes, but only past the
  # gate that says the person is in, and its helper can reach the key; no
  # other login takes one, and with none named polkit asks for the
  # password alone.
  polkit-takes-a-yubikey-touch-once-one-is-named =
    let
      helper = enrolled.config.systemd.services."polkit-agent-helper@".serviceConfig;
      # The auth line just before the touch.
      gate =
        let
          lines = lib.filter (lib.hasPrefix "auth ") (lib.splitString "\n" polkit);
          at = lib.lists.findFirstIndex (lib.hasInfix "pam_u2f.so") null lines;
        in
        if at == null || at == 0 then "" else lib.elemAt lines (at - 1);
      credential = "aGFuZGxl,cHVibGlj,es256,+presence";
      enrolled = bareNixos [
        self.nixosModules.workstation
        { jitsusama.yubikey.administers.someone = [ credential ]; }
      ];
      mappings = enrolled.config.environment.etc.u2f-mappings.text;
      pam = machine: service: machine.config.security.pam.services.${service}.text;
      touches = machine: service: lib.hasInfix "pam_u2f.so" (pam machine service);
      polkit = pam enrolled "polkit-1";
      # The first of its sign-in's lines to ask for the key or the password.
      asksFirst = lib.findFirst (
        line: lib.hasInfix "pam_u2f.so" line || lib.hasInfix "pam_unix.so" line
      ) "" (lib.filter (lib.hasPrefix "auth ") (lib.splitString "\n" polkit));
    in
    holds "polkit-takes-a-yubikey-touch-once-one-is-named"
      "polkit's sign-in doesn't take a named key's touch first from /etc/u2f-mappings, another login takes one, or polkit takes one with no key named."
      (
        touches enrolled "polkit-1"
        && lib.hasInfix "sufficient" asksFirst
        && lib.hasInfix "pam_u2f.so" asksFirst
        && lib.hasInfix "authfile=/etc/u2f-mappings" polkit
        && lib.hasInfix "origin=pam://administrators" polkit
        && lib.hasInfix "[success=ok default=1]" gate
        && lib.hasInfix "pam_exec.so quiet" gate
        && lib.hasInfix "yubikey-once-in" gate
        && !helper.PrivateDevices
        && lib.elem "char-hidraw rw" helper.DeviceAllow
        && mappings == "someone:${credential}\n"
        && !touches enrolled "login"
        && !touches enrolled "sudo"
        && !touches workstation "polkit-1"
      );
}
