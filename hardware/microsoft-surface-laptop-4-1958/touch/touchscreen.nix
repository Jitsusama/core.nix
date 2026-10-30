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
  # Gate5 profile), the ones the driver was qualified with: standard HID
  # reports rather than raw heat maps, and its feature reads left out.
  boot.extraModprobeConfig = ''
    options sl4a_spi_hid raw_mode=N raw_input_beta=Y wire_double_opcode=1 gate3_observe_only=1 skip_std_getfeat=1 std_raw_transition=1 get_noread=0 getfeat_delay_ms=0 std_liveness_ms=0 std_liveness_recover=0 wait_reset_kick_ms=0
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
