# power-profiles-daemon, which sets how hard the CPU and the firmware run:
# performance, balanced or power saver. Balanced leans to performance on AC
# and to battery life on battery by itself. The profile Joel picks is the
# daemon's own state, kept across boots, so nothing here chooses one.
{
  services.power-profiles-daemon.enable = true;

  # The daemon learns from UPower whether the machine is on battery.
  services.upower.enable = true;
}
