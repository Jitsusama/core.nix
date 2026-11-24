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

  # The graphical role's polkit rules, with its tuning chains and
  # noise-cancelled, as polkitd assembles them, run in duktape, polkit's own
  # engine, after polkit's init.js, with an account's manager asking before
  # the seat has a session and after: PipeWire's three units, each chain and
  # each service PipeWire wants in session.slice, by name in session.slice
  # or app.slice, are given real time and a high priority from rtkit, and
  # each grant is logged with what it runs, while anything else of the
  # account's keeps rtkit's own answer, leaving no grant's line, and agents'
  # places are still refused (realtime-test.rules). The slices' stand-in for
  # polkitd serves here too. No chain's name holds a dot today, so one is put
  # among the names, to show a dot stands for itself. Every placeholder is
  # filled, since a cat left unnamed would refuse sound in silence.
  sound-takes-real-time-before-the-seat =
    let
      assembled = graphical.config.environment.etc."polkit-1/rules.d/10-nixos.rules".text;
      dotted = ''"wireplumber","tuning.v2"'';
      rules = builtins.replaceStrings [ ''"wireplumber"'' ] [ dotted ] assembled;
      unfilled = builtins.filter builtins.isList (builtins.split "@[A-Za-z]+@" assembled);
    in
    assert lib.assertMsg (unfilled == [ ]) "polkit's rules hold a placeholder unfilled";
    assert lib.assertMsg (rules != assembled) "realtime.rules no longer names its services as JSON";
    pkgs.runCommandLocal "sound-takes-real-time-before-the-seat" { } ''
      ${lib.getExe' pkgs.duktape "duk"} ${../slices/agents-ask-polkit.rules} \
        ${pkgs.polkit.src}/src/polkitbackend/init.js \
        ${pkgs.writeText "10-nixos.rules" rules} ${./realtime-test.rules}
      touch $out
    '';
}
