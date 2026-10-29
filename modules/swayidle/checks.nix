# The checks of swayidle, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
  lib,
  pkgs,
  ...
}:
{
  # Before sleep the lock is drawn and then the screens go off, even when
  # the lock was late (lock-for-sleep-test.sh), and swayidle runs it before
  # every sleep. Only a niri driving a graphics card turns a screen off, and
  # a VM's niri runs nested, so this holds the order rather than the screen.
  the-screens-go-off-before-sleep =
    let
      beforeSleep = toString (account graphical).services.swayidle.events.before-sleep;
    in
    assert lib.assertMsg (lib.hasSuffix "/bin/lock-for-sleep" beforeSleep)
      "swayidle doesn't turn the screens off before sleep.";
    pkgs.runCommandLocal "the-screens-go-off-before-sleep" { } ''
      ${lib.getExe pkgs.bash} ${./lock-for-sleep-test.sh} ${./lock-for-sleep.sh} $PWD/stand-ins
      touch $out
    '';

  # Once the machine has woken, the screens it went down without come back
  # on, to the lock after a suspend. Only logind can say a sleep ended, so
  # the desktop's VM test runs what comes before it and this holds what
  # comes after.
  the-screens-come-back-after-sleep =
    holds "the-screens-come-back-after-sleep"
      "swayidle doesn't turn the screens back on once the machine wakes."
      (
        lib.hasSuffix " msg action power-on-monitors" (
          toString (account graphical).services.swayidle.events.after-resume
        )
      );

  # The screens go off through the machine's own niri, the one with core's
  # patches, so the account doesn't carry a second one just to ask.
  one-niri =
    let
      niri = lib.getExe graphical.config.programs.niri.package;
      swayidle = (account graphical).services.swayidle;
      commands =
        map (timeout: timeout.command) swayidle.timeouts
        ++ lib.filter lib.isString (lib.attrValues swayidle.events);
    in
    holds "one-niri" "swayidle calls a niri other than the machine's." (
      lib.any (lib.hasInfix "/bin/niri ") commands
      && lib.all (command: !lib.hasInfix "/bin/niri " command || lib.hasPrefix niri command) commands
    );
}
