# The light sensors, set up for the account's backlight daemon (home.nix) to
# hear. udev turns on each one's buffer with its illuminance in it and lets
# the seat's user read it, so the daemon wakes only when a sample arrives,
# which a sensor hub sends when the light changes. iio-sensor-proxy, where a
# machine runs it for its other sensors, is told to leave them alone: it
# would turn their buffers off, and where it can't hear one it reads it on a
# timer.
# systemd-backlight's units save each level at shutdown, but the screen's
# only while the lid is open (saves-while-open.sh), since a shut lid's panel
# reads dark and the next boot would come up at it.
{ config, pkgs, ... }:
let
  savesWhileOpen = pkgs.callPackage ./saves-while-open.nix {
    save = "${config.systemd.package}/lib/systemd/systemd-backlight";
  };

  # A light sensor with a buffer.
  sensor = ''SUBSYSTEM=="iio", TEST=="scan_elements/in_illuminance_en", TEST=="buffer/enable"'';

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
      ACTION=="add|change", ${sensor}, ATTR{buffer/enable}=="0", \
        ATTR{scan_elements/in_illuminance_en}="1", ATTR{buffer/enable}="1"
    '';
  };

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

  systemd.services."systemd-backlight@".serviceConfig.ExecStop = [
    ""
    "${savesWhileOpen}/bin/saves-while-open %i"
  ];
}
