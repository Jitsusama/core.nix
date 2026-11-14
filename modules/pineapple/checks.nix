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
  # resumed, which clears its thread's reply.
  pineapple-judges-each-saying = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in pineapple, its tests included.
  pineapple-passes-clippy = passesClippy "pineapple-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
  # pineapple-slack starts once pi has signed in to Slack, comes back as
  # the new one after a switch and after it fails, runs with what Joel
  # uses, and reads as its own name in ps.
  pineapple-slack-starts-with-the-sign-in =
    let
      home = account (bareNixos [ { home-manager.sharedModules = [ self.homeModules.pineapple ]; } ]);
      unit = home.systemd.user.services.pineapple-slack;
      credentials = "${home.xdg.configHome}/agentic-harness/slack/credentials.json";
    in
    holds "pineapple-slack-starts-with-the-sign-in"
      "pineapple-slack lacks its sign-in start, switch restart, restart, slice or name."
      (
        home.systemd.user.paths.pineapple-slack.Path.PathExists == credentials
        && unit.Unit.X-SwitchMethod == "restart"
        && unit.Service.Restart == "on-failure"
        && unit.Service.Slice == "background.slice"
        # As home-manager takes ExecStart, each start is listed.
        && lib.any (lib.hasSuffix "/bin/pineapple-slack pineapple-slack run") (
          lib.toList unit.Service.ExecStart
        )
      );
}
