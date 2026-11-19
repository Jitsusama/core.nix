# The checks of sessions-come-back, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  bareDarwin,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
let
  # A Mac workstation whose account joins the realms given.
  mac =
    settings:
    account (bareDarwin [
      self.darwinModules.workstation
      { home-manager.users.someone.jitsusama.mycelium = settings; }
    ]);
  realms = {
    realm = "one";
    alsoJoins.two.kit = [ ];
  };
  agentOf = home: home.launchd.agents.sessions-come-back or { enable = false; };
  agent = agentOf (mac realms);
  words = agent.config.ProgramArguments;
  myceliumProgram = lib.getExe self.inputs.mycelium.packages.aarch64-darwin.default;
in
# The program's tests and clippy run on Linux's builders, which a Mac's
# build of it repeats.
onLinux {
  # Its tests run as it builds: a restart sets each pi session held before
  # it waiting and starts it again on the machine in its own folder, one
  # that ended while the machine ran stays ended, one whose folder is gone
  # is left, and one the node refuses is tried again for a while.
  sessions-come-back-after-a-reboot = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in sessions-come-back, its tests included.
  sessions-come-back-passes-clippy = passesClippy "sessions-come-back-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # A Mac whose account joins realms runs the agent as the login starts and
  # each minute, on mycelium and every realm it joins; a Mac in no realm
  # runs none. Linux's roles don't import it.
  a-macs-sessions-come-back-in-each-realm =
    holds "a-macs-sessions-come-back-in-each-realm"
      "A Mac in a realm doesn't run sessions-come-back as the login starts and each minute, on mycelium and each realm it joins, or a Mac in no realm runs it."
      (
        agent.enable
        && lib.length words == 4
        && lib.hasSuffix "/bin/sessions-come-back" (lib.head words)
        && lib.elemAt words 1 == myceliumProgram
        &&
          lib.drop 2 words == [
            "one"
            "two"
          ]
        && agent.config.RunAtLoad
        && agent.config.StartInterval == 60
        && !(agentOf (mac { })).enable
      );
}
