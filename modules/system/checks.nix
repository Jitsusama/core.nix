# The checks of system, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareDarwin,
  bareNixos,
  holds,
  lib,
  macWorkstation,
  passesClippy,
  pkgs,
  self,
  workstation,
  ...
}:
let
  # A workstation of either system whose account's sessions are in a realm.
  inRealm =
    bare: role:
    account (bare [
      role
      {
        home-manager.users.someone.jitsusama.mycelium = {
          realm = "personal";
          links = [ "one" ];
        };
      }
    ]);
in
{
  # system's tests run as it builds: each part judged from its owner's
  # answer as that owner writes it, an owner that fails leaving its part
  # not read, and the screen fitting each width.
  system-judges-each-part = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in system, its tests included.
  system-passes-clippy = passesClippy "system-passes-clippy" (pkgs.callPackage ./package.nix { });
  # The daemon asks mycelium in its account's realm: with two nodes on a
  # machine, mycelium refuses a command naming none, and the machines and
  # mycelium parts both said not read. One in no realm names none.
  system-reads-its-machines-realm =
    let
      linux = inRealm bareNixos self.nixosModules.workstation;
      mac = inRealm bareDarwin self.darwinModules.workstation;
      apart = account workstation;
      said = environment: lib.filter (lib.hasPrefix "MYCELIUM_REALM=") environment;
      macSaid = mac.launchd.agents.system.config.EnvironmentVariables.MYCELIUM_REALM or null;
    in
    holds "system-reads-its-machines-realm"
      "The system daemon asks mycelium in no realm, or another than its account's sessions, on Linux or on a Mac, or names one on a machine in none."
      (
        said linux.systemd.user.services.system.Service.Environment == [ "MYCELIUM_REALM=personal" ]
        && macSaid == "personal"
        && said apart.systemd.user.services.system.Service.Environment == [ ]
      );
  # The daemon serves where the machine's mycelium node reads how it
  # stands, on both kinds of machine: on Linux from the socket systemd
  # holds in the runtime folder, and on a Mac, which has neither, from one
  # its launchd agent listens on in the account's state folder.
  system-serves-where-its-node-reads =
    let
      linux = account workstation;
      mac = account macWorkstation;
      agent = mac.launchd.agents.system;
      words = agent.config.ProgramArguments;
      path = lib.splitString ":" agent.config.EnvironmentVariables.PATH;
    in
    holds "system-serves-where-its-node-reads"
      "The system daemon serves somewhere its machine's mycelium node doesn't read, or a Mac runs no daemon, runs another program, runs it in the foreground, doesn't start it again, or can't find the account's commands it asks."
      (
        linux.systemd.user.sockets.system.Socket.ListenStream == linux.jitsusama.mycelium.healthSocket
        && agent.enable
        && lib.hasSuffix "/bin/system" (lib.head words)
        && lib.elem "${mac.home.profileDirectory}/bin" path
        && lib.elem "/run/current-system/sw/bin" path
        &&
          lib.drop 1 words == [
            "serve"
            "--socket"
            mac.jitsusama.mycelium.healthSocket
          ]
        && lib.hasPrefix "${mac.xdg.stateHome}/" mac.jitsusama.mycelium.healthSocket
        && agent.config.KeepAlive
        && agent.config.ProcessType == "Background"
      );
}
