# The light sensors, set up for the account's backlight daemon (home.nix) to
# hear. udev turns on each one's buffer with its illuminance in it, and its
# colour temperature where it sees colour, and lets the seat's user read it,
# so the daemon wakes only when a sample arrives, which a sensor hub sends
# when the light changes. A colour sensor with no such buffer has its
# threshold event turned on instead, for the daemon to move. iio-sensor-proxy, where a
# machine runs it for its other sensors, is told to leave them alone: it
# would turn their buffers off, and where it can't hear one it reads it on a
# timer.
# systemd-backlight's units save each level at shutdown, but the screen's
# only while the lid is open (saves-while-open.sh), since a shut lid's panel
# reads dark and the next boot would come up at it, and a root service keeps
# the screen's saved level current while the lid is open through the same
# saver, so a shutdown with the lid shut loses nothing set since. The
# session's bus starts
# the program that warms the built-in panel when the daemon first asks it.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  savesWhileOpen = pkgs.callPackage ./saves-while-open.nix {
    save = "${config.systemd.package}/lib/systemd/systemd-backlight";
  };

  # A light sensor with a buffer.
  sensor = ''SUBSYSTEM=="iio", TEST=="scan_elements/in_illuminance_en", TEST=="buffer/enable"'';

  # A colour sensor with no buffer for its light, such as an APDS-9960,
  # whose clear channel says when its count leaves a window the daemon
  # moves (src/windowed.rs).
  windowed = ''SUBSYSTEM=="iio", TEST=="in_intensity_clear_raw", TEST=="events/in_intensity_clear_thresh_either_en"'';
  # Its window's two thresholds, which the daemon sets as the seat's user.
  thresholds = map (side: "/sys%p/events/in_intensity_clear_thresh_${side}_value") [
    "falling"
    "rising"
  ];

  # Before the seat's rules, which give the user what's tagged.
  hear = pkgs.writeTextFile {
    name = "light-sensors-heard";
    destination = "/etc/udev/rules.d/70-light-sensors.rules";
    text = ''
      ACTION!="remove", ${sensor}, TAG+="uaccess"
      # A sensor hub's buffer fills from a trigger named for the device,
      # which can change only while the buffer is off.
      ACTION=="add|change", ${sensor}, ATTR{buffer/enable}=="0", \
        TEST=="trigger/current_trigger", ATTR{trigger/current_trigger}="$attr{name}-dev%n"
      # A channel can be added only while the buffer is off, so a buffer
      # turned on before this rule was here is turned off for it.
      ACTION=="add|change", ${sensor}, TEST=="scan_elements/in_colortemp_en", \
        ATTR{scan_elements/in_colortemp_en}=="0", ATTR{buffer/enable}="0", \
        ATTR{scan_elements/in_colortemp_en}="1", ATTR{scan_elements/in_illuminance_en}="1", \
        ATTR{buffer/enable}="1"
      ACTION=="add|change", ${sensor}, ATTR{buffer/enable}=="0", \
        ATTR{scan_elements/in_illuminance_en}="1", ATTR{buffer/enable}="1"
      # A tenth of a second at four times the gain counts a dim room's light
      # finely and a bright one's short of the channel's top. Its event is
      # turned on with a window no count leaves, so nothing wakes before
      # the daemon sets its own, and its thresholds go to the users' group,
      # which the seat's user is in from the start of the session, as a
      # group added now would be only from the next.
      ACTION!="remove", ${windowed}, TAG+="uaccess"
      ACTION=="add|change", ${windowed}, ATTR{in_intensity_integration_time}="0.1", \
        ATTR{in_intensity_scale}="4", \
        ATTR{events/in_intensity_clear_thresh_falling_value}="0", \
        ATTR{events/in_intensity_clear_thresh_rising_value}="65535", \
        ATTR{events/in_intensity_clear_thresh_either_en}="1", \
        RUN+="${pkgs.coreutils}/bin/chgrp users ${lib.concatStringsSep " " thresholds}", \
        RUN+="${pkgs.coreutils}/bin/chmod g+w ${lib.concatStringsSep " " thresholds}"
    '';
  };

  # wl-gammarelay-rs, which warms the built-in panel for the daemon, is
  # started by the session's bus as the account's unit (home.nix) when the
  # daemon first sets a warmth, so it runs only once the room or the night
  # first warms the screen.
  relayStarted = pkgs.runCommand "wl-gammarelay-activation" { } ''
    mkdir -p $out/share/dbus-1/services
    printf '[D-BUS Service]\nName=%s\nExec=%s\nSystemdService=%s\n' \
      rs.wl-gammarelay ${pkgs.coreutils}/bin/false wl-gammarelay.service \
      > $out/share/dbus-1/services/rs.wl-gammarelay.service
  '';

  # After iio-sensor-proxy's own rules, which say what it may drive.
  unclaimed = pkgs.writeTextFile {
    name = "light-sensors-unclaimed";
    destination = "/etc/udev/rules.d/81-light-sensors.rules";
    text = ''
      ${sensor}, ENV{IIO_SENSOR_PROXY_TYPE}=""
    '';
  };
in
{
  services.udev.packages = [
    hear
    unclaimed
  ];

  services.dbus.packages = [ relayStarted ];

  # A switch reloads udev's rules but replays no event, so a sensor set up
  # by older rules would keep its buffer as it was until the next boot.
  # This replays the sensors' change event after boot's own, and whenever
  # the rules change.
  systemd.services.light-sensors-heard = {
    description = "Sets up the light sensors by udev's current rules";
    wantedBy = [ "multi-user.target" ];
    after = [ "systemd-udev-trigger.service" ];
    restartTriggers = [ hear ];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = "${config.systemd.package}/bin/udevadm trigger --action=change --subsystem-match=iio";
    };
  };

  # Saving only at shutdown lost every level set since the last shutdown with
  # the lid open, for a machine shut down with it shut. So the panel's saved
  # level is kept current too, through the same saver, each time its
  # brightness holds after a change (src/keep.rs).
  systemd.services.backlight-keeps-the-open-level = {
    description = "Keeps the panel's saved brightness current while the lid is open";
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      ExecStart = "${
        pkgs.callPackage ./package.nix { }
      }/bin/backlight --keeps-the-open-level ${savesWhileOpen}/bin/saves-while-open";
      Restart = "on-failure";
      ProtectHome = true;
      PrivateNetwork = true;
      PrivateTmp = true;
    };
  };

  systemd.services."systemd-backlight@".serviceConfig.ExecStop = [
    ""
    "${savesWhileOpen}/bin/saves-while-open %i"
  ];
}
