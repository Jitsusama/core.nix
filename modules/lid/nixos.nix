# The lid: a laptop shut on battery and away from a monitor suspends, and one
# shut on the charger or at a desk keeps running, so work left going
# overnight goes on. logind looks at a shut lid again whenever anything
# happens, so unplugging the charger with the lid still shut suspends it,
# as it would in a bag.
#
# logind tells a desk from anywhere else by counting monitors, but only
# those whose connectors are driving them, so once going idle turns the
# monitor off, a laptop shut at a desk reads as undocked, and logind, which
# looks again at a shut lid whenever it wakes, suspends it minutes later.
#
# A monitor that's plugged in still reads connected with its screen off. So
# lid-docked holds logind's lock against the lid's handling while any
# connector but the laptop's own screen reads connected. udev looks whenever a
# graphics card appears or says a monitor came or went, starting the lock's
# holder or stopping it. It asks each connector to look again first, since
# amdgpu says a monitor went before its connector reads disconnected. Unplugging the monitor with the lid shut lets go of
# the lock, and the lid suspends the laptop on battery as it would anywhere
# else. A
# suspend asked for, from the power key or the shell, isn't held.
{ lib, pkgs, ... }:
let
  docked = pkgs.callPackage ./docked/package.nix { };
in
{
  # Suspend on battery is logind's own, said here beside its exception.
  services.logind.settings.Login = {
    HandleLidSwitch = "suspend";
    HandleLidSwitchExternalPower = "ignore";
  };

  # Root's, since logind gives the lid's lock to a service with no session
  # only when it's root's.
  systemd.services.lid-docked = {
    description = "Keeps a laptop shut at a desk running while a monitor is plugged in";
    after = [ "systemd-logind.service" ];
    serviceConfig = {
      Type = "exec";
      ExecStart = lib.escapeShellArgs [
        "${pkgs.systemd}/bin/systemd-inhibit"
        "--what=handle-lid-switch"
        "--who=lid-docked"
        "--why=A monitor is plugged in"
        "--mode=block"
        "${pkgs.coreutils}/bin/sleep"
        "infinity"
      ];
      NoNewPrivileges = true;
      PrivateNetwork = true;
      PrivateTmp = true;
      ProtectHome = true;
      ProtectSystem = "strict";
      MemoryDenyWriteExecute = true;
      SystemCallArchitectures = "native";
    };
  };

  # A card says a monitor came or went by a change on the card itself, not its
  # connectors, and it appears again in the coldplug after the initrd.
  services.udev.extraRules = ''
    ACTION=="add|change", SUBSYSTEM=="drm", ENV{DEVTYPE}=="drm_minor", KERNEL=="card[0-9]*", RUN+="${lib.getExe docked} ${pkgs.systemd}/bin/systemctl lid-docked.service"
  '';
}
