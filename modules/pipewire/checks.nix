# The checks of pipewire, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  graphical,
  holds,
  lib,
  pkgs,
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

  # A workstation's polkit rules, as polkitd assembles them, run in duktape,
  # polkit's own engine, after polkit's init.js, with the account's manager
  # asking before the seat has a session: PipeWire's units in session.slice
  # and app.slice, and what runs below them, are given real time and a high
  # priority from rtkit, while an app's scope, the system's units and a near
  # name keep rtkit's own answer and agents' places are still refused
  # (realtime-test.rules). The slices' stand-in for polkitd serves here too.
  sound-takes-real-time-before-the-seat =
    let
      rules = graphical.config.environment.etc."polkit-1/rules.d/10-nixos.rules".text;
    in
    pkgs.runCommandLocal "sound-takes-real-time-before-the-seat" { } ''
      ${lib.getExe' pkgs.duktape "duk"} ${../slices/agents-ask-polkit.rules} \
        ${pkgs.polkit.src}/src/polkitbackend/init.js \
        ${pkgs.writeText "10-nixos.rules" rules} ${./realtime-test.rules}
      touch $out
    '';
}
