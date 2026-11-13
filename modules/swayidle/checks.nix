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

  # lock-screen, lock-for-sleep and swayidle-away answer --help and refuse
  # a word they don't know, asking nothing of the shell, niri or swayidle
  # (asked-wrongly-test.sh), since lock-screen once took --help for its
  # reason and locked the screen.
  locking-asked-wrongly-does-nothing =
    pkgs.runCommandLocal "locking-asked-wrongly-does-nothing" { }
      ''
        ${lib.getExe pkgs.bash} ${./asked-wrongly-test.sh} ${./lock-screen.sh} ${./lock-for-sleep.sh} \
          ${./away.sh} $PWD/stand-ins
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

  # logind hears the session is idle as the screens go off, which is all idle
  # sleep waits for. Only a niri driving a graphics card turns a screen off,
  # so this holds the two together, and idle-sleep's VM test what logind
  # does once it hears.
  the-session-goes-idle-as-the-screens-go-off =
    let
      swayidle = (account graphical).services.swayidle;
      service = (account graphical).systemd.user.services.swayidle-away.Service;
      # Home Manager keeps each Exec line as a list.
      away = lib.splitString " " (toString service.ExecStart);
      hint = lib.lists.findFirstIndex (arg: arg == "idlehint") null swayidle.extraArgs;
      idleAt = lib.elemAt swayidle.extraArgs (hint + 1);
    in
    holds "the-session-goes-idle-as-the-screens-go-off"
      "swayidle doesn't tell logind the session is idle as the screens go off."
      (hint != null && lib.length away == 4 && idleAt == lib.elemAt away 3);

  # A switch the follower asked for waits while someone is at the machine
  # (../update-to-main/nixos.nix), which this marks from any key or touch
  # until two minutes pass without one, and never from swayidle-away's start,
  # which a switch brings while Joel is away.
  someone-at-the-machine-is-marked =
    let
      service = (account graphical).systemd.user.services.swayidle-away.Service;
      marks = line: lib.hasInfix "someone-at-the-machine" (toString line);
      away = lib.splitString " " (toString service.ExecStart);
      script = builtins.readFile ./away.sh;
    in
    holds "someone-at-the-machine-is-marked"
      "swayidle doesn't mark someone at the machine for a followed switch to wait on."
      (
        lib.length away == 4
        && lib.elemAt away 1 == "120"
        && lib.hasInfix "timeout 1 true resume \"touch '$here'\"" script
        && lib.hasInfix "timeout \"$mark\" \"rm -f '$here'\"" script
        && !(service ? ExecStartPre)
        && marks service.ExecStopPost
      );

  # What happens as Joel steps away asks logind nothing, so no hold of
  # logind's lock against going idle stops the lock: the swayidle that hears
  # logind has no timeouts, and swayidle-away is given no logind event, which
  # is all that makes a swayidle ask logind. mycelium's hold stopped the lock
  # while Joel was away for as long as it held work (2026-11-10). The
  # desktop's VM test locks with such a hold up and keeps it off for Keep
  # Awake; this holds the shape that makes it so on every machine.
  stepping-away-asks-logind-nothing =
    let
      # What it runs swayidle with, after its comments and arguments.
      away = lib.last (lib.splitString "exec swayidle" (builtins.readFile ./away.sh));
      events = [
        "before-sleep"
        "after-resume"
        "lock "
        "unlock"
        "idlehint"
      ];
    in
    holds "stepping-away-asks-logind-nothing"
      "something that locks as Joel steps away waits on logind's idle holds."
      (
        (account graphical).services.swayidle.timeouts == [ ]
        && lib.all (event: !lib.hasInfix event away) events
        && lib.hasInfix "screen-kept || lock-screen" away
        && lib.hasInfix "screen-kept || niri msg action power-off-monitors" away
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
