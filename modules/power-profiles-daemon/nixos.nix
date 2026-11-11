# power-profiles-daemon, which sets how hard the CPU and the firmware run:
# performance, balanced or power saver. The profile follows the charger:
# performance on the charger where the machine has that profile, and balanced on
# battery, which the daemon itself leans towards battery life. Below UPower's
# low mark, 20%, it goes to power saver until the charger is back (follow.sh
# says how). A profile picked by hand
# holds until the next of those changes. The daemon leaves a battery that
# charges adaptively or to custom limits as it is, so the profile never changes
# how the battery charges.
#
# UPower also decides what happens as the battery runs out. It hibernates the
# machine at 5%, warning at 20% and 10% as the desktop's battery does.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  service = "power-profile-follows-the-charger";
  low = toString config.services.upower.percentageLow;

  chosen = config.jitsusama.power-profiles-daemon.package;

  # The daemon chosen, but for its unit's order after multi-user.target,
  # which a later file can't take away. That target waits for whatever waits
  # for the network, so on a boot whose Wi-Fi is slow or missing the daemon
  # started half a minute late, and the shell, which asks it for the profile
  # as it starts, waited 25 s for an answer.
  daemon = pkgs.symlinkJoin {
    name = "power-profiles-daemon-${chosen.version}";
    paths = [ chosen ];
    inherit (chosen) meta;
    postBuild = ''
      unit=lib/systemd/system/power-profiles-daemon.service
      rm "$out/$unit"
      grep -v '^After=' ${chosen}/$unit > "$out/$unit"
    '';
  };
in
{
  # The daemon to run, which a machine's hardware may patch.
  options.jitsusama.power-profiles-daemon.package =
    lib.mkPackageOption pkgs "power-profiles-daemon"
      { };

  config = {
    services.power-profiles-daemon = {
      enable = true;
      package = daemon;
    };

    # The daemon learns from UPower whether the machine is on battery.
    services.upower = {
      enable = true;
      # UPower's own 2% leaves too little. It waits 20 s before acting, and
      # under a build's load 1% lasts about a minute, so a laptop at 2% went
      # flat before anything happened. 5% leaves minutes under load.
      percentageLow = 20;
      percentageCritical = 10;
      percentageAction = 5;
      # A laptop kept awake runs on with its lid shut and work open, so the
      # battery running out hibernates it, keeping the work, rather than
      # powering it off (tests/vm/disks/hibernate-vm-test.py proves it with
      # Keep Awake's locks held). Where it can't hibernate, as with no swap
      # file or none big enough, UPower powers it off instead.
      criticalPowerAction = "Hibernate";
    };

    systemd.services.${service} = {
      description = "Set the power profile for the charger and the battery";
      # UPower says whether the machine is on battery; asked before it's up,
      # the answer would read as the charger.
      after = [
        "power-profiles-daemon.service"
        "upower.service"
      ];
      wants = [
        "power-profiles-daemon.service"
        "upower.service"
      ];
      # Started with the daemon, whether at boot or by D-Bus.
      wantedBy = [ "power-profiles-daemon.service" ];
      path = [
        config.systemd.package
        (lib.getBin pkgs.glib)
      ];
      serviceConfig = {
        ExecStart = "${lib.getExe pkgs.bash} ${./follow.sh} ${low}";
        Restart = "on-failure";
        RestartSec = 10;
        # Where follow.sh says what it chose, for the shell to tell a
        # profile picked by hand from its own. Kept over a restart, so the
        # shell's watch on the file holds.
        RuntimeDirectory = "power-profile";
        RuntimeDirectoryPreserve = "yes";
      };
    };
  };
}
