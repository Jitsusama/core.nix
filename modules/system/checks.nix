# The checks of system, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  holds,
  lib,
  macWorkstation,
  passesClippy,
  pkgs,
  workstation,
  ...
}:
{
  # system's tests run as it builds: each part judged from its owner's
  # answer as that owner writes it, an owner that fails leaving its part
  # not read, and the screen fitting each width.
  system-judges-each-part = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in system, its tests included.
  system-passes-clippy = passesClippy "system-passes-clippy" (pkgs.callPackage ./package.nix { });
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
    in
    holds "system-serves-where-its-node-reads"
      "The system daemon serves somewhere its machine's mycelium node doesn't read, or a Mac runs no daemon, runs it in the foreground, or doesn't start it again."
      (
        linux.systemd.user.sockets.system.Socket.ListenStream == linux.jitsusama.mycelium.healthSocket
        && agent.enable
        && lib.drop 1 words == [
          "serve"
          "--socket"
          mac.jitsusama.mycelium.healthSocket
        ]
        && lib.hasPrefix "${mac.xdg.stateHome}/" mac.jitsusama.mycelium.healthSocket
        && agent.config.KeepAlive
        && agent.config.ProcessType == "Background"
      );
}
