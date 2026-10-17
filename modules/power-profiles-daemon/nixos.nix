# power-profiles-daemon, which sets how hard the CPU and the firmware run:
# performance, balanced or power saver. The profile follows the charger:
# performance on mains power where the machine has that profile, and balanced on
# battery, which the daemon itself leans towards battery life. Below UPower's
# low mark, 20%, it goes to power saver until the charger is back, as Windows'
# energy saver does by default (follow.sh says how). A profile picked by hand
# holds until the next of those changes. The daemon leaves a battery that
# charges adaptively or to custom limits as it is, so the profile never changes
# how the battery charges.
#
# UPower also decides what happens as the battery runs out. It powers the
# machine off at 5%, warning at 20% and 10% as the desktop's battery does.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  service = "power-profile-follows-the-charger";
  low = toString config.services.upower.percentageLow;
in
{
  services.power-profiles-daemon.enable = true;

  # The daemon learns from UPower whether the machine is on battery.
  services.upower = {
    enable = true;
    # UPower's own 2% leaves too little. It waits 20 s before acting, and
    # under a build's load 1% lasts about a minute, so a laptop at 2% went
    # flat before anything happened. 5% leaves minutes under load.
    percentageLow = 20;
    percentageCritical = 10;
    percentageAction = 5;
    # Hibernation hasn't been proven on these machines, and UPower skips
    # any sleep a program's lock holds off and powers off anyway, so it
    # says what happens.
    criticalPowerAction = "PowerOff";
  };

  systemd.services.${service} = {
    description = "Set the power profile for the charger and the battery";
    # UPower says whether the machine is on battery; asked before it's up,
    # the answer would read as mains power.
    after = [
      "power-profiles-daemon.service"
      "upower.service"
    ];
    wants = [
      "power-profiles-daemon.service"
      "upower.service"
    ];
    # Started with the daemon, whether at boot or by D-Bus. The daemon starts
    # after multi-user.target, so anything that target waits for can't wait
    # for the daemon too: that loop drops both at boot.
    wantedBy = [ "power-profiles-daemon.service" ];
    path = [
      config.systemd.package
      pkgs.power-profiles-daemon
      (lib.getBin pkgs.glib)
    ];
    serviceConfig = {
      ExecStart = "${lib.getExe pkgs.bash} ${./follow.sh} ${low}";
      Restart = "on-failure";
      RestartSec = 10;
    };
  };
}
