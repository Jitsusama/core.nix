# The checks of pineapple, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
onLinux {
  # Its tests run as it builds: every saying in sayings.json, the real
  # messages of 13 November and 20 October among them, is judged the safe
  # word, a resume, an ask to stop or a near miss (no tag, a quote, talked
  # about, a joke, a bot) as the table says, and reaches what its words
  # and the place it was said in name, or everything, said so; and Slack's
  # answers become messages, oldest first, a bot's told apart, with the
  # thread each is in, and a stop's record is written whole and once; and
  # pineapple, Joel's own command, lists the stops standing, refuses a
  # resume where pi says it runs, and otherwise marks what a scope names
  # resumed, which clears its thread's reply; and pineapple serve serves
  # a live stop and alert as rows in the launcher's words, writes a shadow
  # one down only, and refuses a press from anything but Joel's shell.
  pineapple-judges-each-saying = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in pineapple, its tests included.
  pineapple-passes-clippy = passesClippy "pineapple-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
  # pineapple-slack starts at login, reads pi's Slack sign-in from the
  # keyring as slack-unreads does, comes back as the new one after a
  # switch and after it fails, runs in the background slice, and reads as
  # its own name in ps.
  pineapple-slack-starts-with-the-sign-in =
    let
      home = account (bareNixos [ { home-manager.sharedModules = [ self.homeModules.pineapple ]; } ]);
      unit = home.systemd.user.services.pineapple-slack;
    in
    holds "pineapple-slack-starts-with-the-sign-in"
      "pineapple-slack lacks its login start, keyring, switch restart, restart, slice or name."
      (
        lib.elem "default.target" unit.Install.WantedBy
        && !(home.systemd.user.paths ? pineapple-slack)
        && lib.any (lib.hasPrefix "SIGN_IN_KEYRING=/nix/store/") (lib.toList unit.Service.Environment)
        && unit.Unit.X-SwitchMethod == "restart"
        && unit.Service.Restart == "on-failure"
        && unit.Service.Slice == "background.slice"
        # As home-manager takes ExecStart, each start is listed.
        && lib.any (lib.hasSuffix "/bin/pineapple-slack pineapple-slack run") (
          lib.toList unit.Service.ExecStart
        )
      );
  # pineapple, the daemon showing stops in the launcher, is started at
  # login for its socket, which systemd holds where the launcher looks,
  # knocks with notify-send, and reads its words from the launcher's
  # vocabulary.
  pineapple-serves-the-launcher-from-its-socket =
    let
      home = account (bareNixos [ { home-manager.sharedModules = [ self.homeModules.pineapple ]; } ]);
      unit = home.systemd.user.services.pineapple;
      socket = home.systemd.user.sockets.pineapple;
      command = lib.concatStringsSep " " (lib.toList unit.Service.ExecStart);
    in
    holds "pineapple-serves-the-launcher-from-its-socket"
      "pineapple lacks its socket, login start, serve command, notice or words."
      (
        socket.Socket.ListenStream == "%t/daemons/pineapple.sock"
        && lib.elem "pineapple.socket" unit.Unit.Requires
        && lib.elem "default.target" unit.Install.WantedBy
        && lib.hasInfix "/bin/pineapple pineapple serve --socket" command
        && lib.hasInfix "/bin/notify-send" command
        && lib.hasInfix "words.json" command
      );
}
