# An external monitor's own controls, over DDC/CI: its brightness, contrast,
# input and whatever else its menu holds, set by the person at the machine
# with ddcutil, or by a program of theirs talking to the bus. A monitor
# answers on its connector's I2C bus, which the kernel offers as /dev/i2c-N
# only with i2c-dev loaded, and which only root can open.
#
# So the person at the seat is given the buses that reach external
# DisplayPort screens and nothing else: a DisplayPort connector's AUX
# channel, which sits under the connector in sysfs, and an MST branch's,
# which the kernel names DPMST, as a monitor with a hub behind USB-C has. A
# laptop's own panel takes no DDC/CI and keeps its AUX channel root's (its
# connector is eDP, not DP), as do the GPU's other buses and the touchpad's
# and sensors' I2C controllers.
{ pkgs, ... }:
let
  rules = pkgs.writeTextDir "lib/udev/rules.d/70-monitor-controls.rules" ''
    SUBSYSTEM=="i2c-dev", KERNELS=="card[0-9]*-DP-[0-9]*", TAG+="uaccess"
    SUBSYSTEM=="i2c-dev", ATTR{name}=="DPMST", TAG+="uaccess"
  '';
in
{
  boot.kernelModules = [ "i2c-dev" ];
  environment.systemPackages = [ pkgs.ddcutil ];
  # Before systemd's 73-seat-late.rules, which hands a device tagged
  # uaccess to whoever sits at the seat, so the tag has to be on by then.
  services.udev.packages = [ rules ];
}
