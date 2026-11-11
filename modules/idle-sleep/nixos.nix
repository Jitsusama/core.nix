# Idle sleep: a laptop left idle on battery sleeps, and one on the charger
# never sleeps by itself. Idle is logind's: every session idle, and nothing
# holding logind's lock against going idle. The desktop's session goes idle
# once swayidle has turned the screens off (../swayidle/home.nix), and a call
# or a video in Chrome keeps it from going idle through Wayland's own
# inhibitor, which niri honours. Keep Awake holds logind's lock against idling,
# so nothing locks either; mycelium, while it holds work, holds its lock
# against sleep alone, so the machine still goes idle and locks but never
# sleeps under it.
#
# It sleeps five minutes after the screens go off, fifteen after the last key
# or touch, the battery's default in GNOME too. A key a minute after the
# screens go dark still wakes them at once, to the lock rather than through a
# wake, and a laptop left open spends about 0.5 Wh of its battery in those five
# minutes rather than its whole charge over six hours.
#
# logind's idle action is one for the battery and the charger alike, unlike
# its lid's. So logind's settings here are the charger's, which only lock, as
# the lock screen already has, and follow.sh adds the battery's, to suspend,
# as a drop-in of logind's own in /run, and takes it away on the charger,
# reloading logind each time. The charger's action is to lock rather than to
# ignore, since logind sets its idle timer only as it starts, and ignoring
# stops the timer for good, where no reload starts it again. A lock while the
# session stays idle does nothing.
#
# The charger is read as logind reads it for the lid
# (HandleLidSwitchExternalPower), by systemd-ac-power, and a reading that
# fails counts as the charger, so a machine that can't tell never sleeps by
# itself. udev restarts the follower whenever a power supply changes,
# stopping a run still going, and each run reloads logind whenever logind's
# own idle action isn't yet the one it read for, so the newest reading is
# always the last one acted on.
#
# Holding logind's lock against idling on the charger would be simpler, but
# swayidle drops every timeout while anything holds it, so the screens would
# never lock or go off on the charger.
#
# logind arms its idle timer as it starts, so a switch that brings this to a
# running machine takes effect from its next boot.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  follower = "idle-sleep-follows-the-charger";
  settings = "/run/systemd/logind.conf.d";
  systemctl = "${config.systemd.package}/bin/systemctl";
in
{
  services.logind.settings.Login = {
    IdleAction = "lock";
    IdleActionSec = "5min";
  };
  # A switch that changes them reloads logind, as the lid's own settings do
  # (../lid/nixos.nix says why this trigger does).
  systemd.services.systemd-logind.restartTriggers = [
    config.environment.etc."systemd/logind.conf".source
  ];

  # Where logind reads its drop-ins from at runtime, made for the follower,
  # which can write nowhere else.
  systemd.tmpfiles.rules = [ "d ${settings} 0755 root root -" ];

  systemd.services.${follower} = {
    description = "Lets an idle machine sleep on battery and only lock on the charger";
    # It reloads logind, which has to be running to hear it.
    after = [ "systemd-logind.service" ];
    wantedBy = [ "multi-user.target" ];
    # A charger plugged in or out, or a USB-C supply settling, sends a burst
    # of power_supply events, each a restart, past systemd's five starts in
    # ten seconds. The unit was then refused, and the burst's last reading,
    # the one that counts, never acted on. Each run is a reading and at most
    # one reload, so it takes every one.
    startLimitIntervalSec = 0;
    path = [ config.systemd.package ];
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${lib.getExe pkgs.bash} ${./follow.sh} ${settings}/idle-sleep.conf";
      NoNewPrivileges = true;
      PrivateNetwork = true;
      PrivateTmp = true;
      ProtectHome = true;
      ProtectSystem = "strict";
      ReadWritePaths = [ settings ];
      MemoryDenyWriteExecute = true;
      SystemCallArchitectures = "native";
    };
  };

  # Only a charger's events, never a battery's: a battery charging says its
  # level every few seconds, which restarted this about 35 times a minute and
  # never changed the reading, since a charger coming or going sends its own.
  services.udev.extraRules = ''
    SUBSYSTEM=="power_supply", ATTR{type}!="Battery", RUN+="${systemctl} --no-block restart ${follower}.service"
  '';
}
