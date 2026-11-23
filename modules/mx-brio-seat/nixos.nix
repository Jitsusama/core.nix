# The MX Brio as the seat's own camera, opened by one program at a time,
# while ../mx-brio's relay gives no picture (its header says why). A machine
# that shared the Brio before keeps it hidden from the seat until it's
# replugged, so a switch bringing this in, and every boot after, gives back
# any camera node still held for a relay (give-back.sh), and nobody has to
# unplug it. A machine importing ../mx-brio drops this, choosing the relay.
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
      # A switch waits on it, and udevadm's settling has no bound of its
      # own; a replay takes seconds, so a minute is a wedged udev.
      TimeoutStartSec = "1min";
    };
  };
}
