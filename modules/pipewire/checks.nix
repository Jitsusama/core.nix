# The checks of pipewire, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  graphical,
  holds,
  lib,
  ...
}:
{
  # A call's audio thread asks for SCHED_RR 8 in Chrome and Electron apps,
  # and gets it only if the account's limit reaches 8. At 11 or more,
  # PipeWire takes its priority from the limit rather than rtkit's 20, so
  # anything an app sets could rank beside it. With no time limit, a
  # real-time thread that spins is never stopped. None of these says so.
  call-audio-runs-in-real-time =
    let
      inherit (graphical.config) systemd;
      manager = systemd.services."user@".serviceConfig;
      units = systemd.user.settings.Manager;
      priority = lib.toInt manager.LimitRTPRIO;
    in
    holds "call-audio-runs-in-real-time"
      "The account can't take real-time priority 8, can take 11 or more, or has no real-time time limit."
      (
        priority >= 8
        && priority < 11
        && (manager.LimitRTTIME or "infinity") != "infinity"
        && (units.DefaultLimitRTPRIO or null) == manager.LimitRTPRIO
        && (units.DefaultLimitRTTIME or null) == manager.LimitRTTIME
      );
}
