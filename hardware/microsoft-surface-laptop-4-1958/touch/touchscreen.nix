# The touchscreen and pen. The panel, MSHW0231, speaks HID over SPI behind
# AMD's SPI controller AMDI0060, and mainline drives neither: its spi-amd
# matches only later AMD SPI blocks, and it has no HID-over-SPI transport for
# this panel. avgbob's SL4A_TouchScreen drives both, out of the kernel's tree
# (package.nix).
#
# Its modules declare no aliases, so nothing loads them while the system
# starts. A unit loads them once it's up, as the driver's own installer does:
# loaded as the devices appeared at boot, an earlier version once left a black
# screen, and loaded late a hang still leaves a login to recover from.
{ config, pkgs, ... }:
let
  touchscreen = config.boot.kernelPackages.callPackage ./package.nix { };
in
{
  boot.extraModulePackages = [ touchscreen ];

  # The panel's settings as the installer writes them for this model (its
  # Gate5 profile), the ones the driver was qualified with: the standard HID
  # start, then a switch to the panel's heat maps, from which the driver
  # tracks every finger (std_raw_transition, raw_input_beta), and its feature
  # reads left out. The panel sends those about 100 times a second while
  # touched, and on some starts untouched too, which the driver's rest stops
  # (package.nix). Its stream watchdog is off too: in raw mode
  # it re-inits a stalled stream, but in standard mode it only logs, and each
  # heat map re-arms it, so once the panel streamed them it wrote a line
  # every 2 s, 1822 in three hours with the lid shut and nothing touching.
  boot.extraModprobeConfig = ''
    options sl4a_spi_hid raw_mode=N raw_input_beta=Y wire_double_opcode=1 gate3_observe_only=1 skip_std_getfeat=1 std_raw_transition=1 get_noread=0 getfeat_delay_ms=0 std_liveness_ms=0 std_liveness_recover=0 wait_reset_kick_ms=0 stream_watchdog_ms=0
  '';

  # The touchscreen's axes run 0 to 32767, but its report descriptor gives
  # them 112 and 198 units a millimetre, a 292 by 165 mm screen. The pen,
  # over the same glass, reads 282 by 189 mm, which fits a 13.5-inch 3:2
  # panel. These resolutions give the touchscreen the pen's size, so what
  # reads a touch in millimetres measures it right.
  services.udev.extraHwdb = ''
    evdev:name:MSHW0231 Touchscreen:*
     EVDEV_ABS_00=::116
     EVDEV_ABS_01=::173
     EVDEV_ABS_35=::116
     EVDEV_ABS_36=::173
  '';

  # The controller's module first, which finds the panel and names it, then
  # the panel's.
  systemd.services.surface-touchscreen = {
    description = "Loads the touchscreen's driver once the system is up";
    after = [ "multi-user.target" ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = "${pkgs.kmod}/bin/modprobe --all sl4a_spi_amd sl4a_spi_hid";
    };
  };
}
