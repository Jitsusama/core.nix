# The MX Brio as the seat's own camera, opened by one program at a time,
# while ../mx-brio's relay gives no picture (its header says why). A machine
# that shared the Brio before keeps it hidden from the seat until it's
# replugged, so a switch bringing this in, and every boot after, gives back
# any of the Brio's nodes still held for a relay (give-back.sh), and nobody
# has to unplug it. It never fails the switch, which would send a machine
# that updates itself back to the sharing. A machine importing ../mx-brio
# drops this, choosing the relay.
{ config, ... }:
{
  systemd.services.mx-brio-seat = {
    description = "Give the seat back an MX Brio a relay held";
    wantedBy = [ "multi-user.target" ];
    after = [
      "systemd-udevd.service"
      "systemd-udev-trigger.service"
    ];
    # The running systemd's udevadm, which speaks to its own udevd.
    path = [ config.systemd.package ];
    script = builtins.readFile ./give-back.sh;
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      # A switch waits on it. give-back.sh bounds each of its two waits on
      # udev at 25 s, so it ends well inside this, and a timeout, which
      # would fail the switch, means the script itself hung.
      TimeoutStartSec = "2min";
    };
  };
}
