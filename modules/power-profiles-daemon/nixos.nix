# power-profiles-daemon, which sets how hard the CPU and the firmware run:
# performance, balanced or power saver. The profile follows the charger:
# performance on mains power where the machine has that profile, and balanced
# on battery, which the daemon itself leans towards battery life. A profile picked by hand holds until the charger
# is next plugged in or pulled out. The daemon leaves a battery that charges
# adaptively or to custom limits as it is, so the profile never changes how
# the battery charges.
{ pkgs, ... }:
let
  service = "power-profile-follows-the-charger";
in
{
  services.power-profiles-daemon.enable = true;

  # The daemon learns from UPower whether the machine is on battery.
  services.upower.enable = true;

  systemd.services.${service} = {
    description = "Set the power profile for the charger";
    after = [ "power-profiles-daemon.service" ];
    wants = [ "power-profiles-daemon.service" ];
    # Started with the daemon, whether at boot or by D-Bus. The daemon starts
    # after multi-user.target, so anything that target waits for can't wait
    # for the daemon too: that loop drops both at boot.
    wantedBy = [ "power-profiles-daemon.service" ];
    path = [ pkgs.power-profiles-daemon ];
    serviceConfig.Type = "oneshot";
    script = ''
      profile=balanced
      # Without the firmware's or the CPU's support, the daemon offers no
      # performance profile, and asking for one fails.
      if powerprofilesctl list | grep -q 'performance:'; then
        for supply in /sys/class/power_supply/*; do
          if [ "$(cat "$supply/type")" = Mains ] && [ "$(cat "$supply/online")" = 1 ]; then
            profile=performance
          fi
        done
      fi
      powerprofilesctl set "$profile"
      echo "power profile: $profile"
    '';
  };

  # Mains power coming or going is a change event on its supply.
  services.udev.extraRules = ''
    SUBSYSTEM=="power_supply", ATTR{type}=="Mains", ACTION=="change", RUN+="${pkgs.systemd}/bin/systemctl start --no-block ${service}.service"
  '';
}
