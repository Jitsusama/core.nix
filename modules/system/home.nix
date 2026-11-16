# system says how the whole system stands at once: the machines of the
# realm, the mycelium, the home network and the code (src/lib.rs says how).
# It asks each part's own command, mycelium, net and repos, as they're found
# on the account's path, so it keeps no copy of theirs, and a machine
# without one of them says that part wasn't read.
#
# On Linux it's also a daemon, holding the record ready over
# $XDG_RUNTIME_DIR/daemons/system.sock in the shell's daemon interface, for
# the system page and the launcher's home rows (src/serve.rs says how).
{
  config,
  lib,
  pkgs,
  ...
}:
let
  system = pkgs.callPackage ./package.nix { };
in
{
  home.packages = [ system ];

  # So an agent asked how everything stands answers from the owners, in
  # words, worst first, as the terminal does.
  home.file.".pi/agent/skills/system/SKILL.md".source = ./SKILL.md;

  systemd.user = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    # In background.slice: it asks the owners on a pace whether or not
    # anyone watches, and what it holds is a breath old at worst, so the
    # work Joel is doing comes first.
    services.system = {
      Unit = {
        Description = "Holds how the whole system stands ready for its views";
        # Restarted after a switch reloads the units, it comes back as the
        # new one.
        X-SwitchMethod = "restart";
        # Its socket is systemd's, handed to it as it starts.
        Requires = [ "system.socket" ];
        After = [ "system.socket" ];
      };
      Service = {
        # Run by its name, so ps -ef reads system rather than a store path
        # (docs/daemons.md, Shown).
        ExecStart = "@${lib.getExe system} system serve";
        # The owners it asks are the account's own commands, found where
        # its login shell finds them.
        Environment = [
          "PATH=${
            lib.concatStringsSep ":" [
              "/etc/profiles/per-user/${config.home.username}/bin"
              "${config.home.homeDirectory}/.nix-profile/bin"
              "/run/current-system/sw/bin"
            ]
          }"
        ];
        Restart = "on-failure";
        RestartSec = 5;
        Slice = "background.slice";
      };
      Install.WantedBy = [ "default.target" ];
    };

    # Its socket, held by systemd whether or not the daemon runs, so a view
    # asking while it starts waits on it rather than finding no socket.
    sockets.system = {
      Unit.Description = "Where the views ask the system daemon";
      Socket = {
        ListenStream = "%t/daemons/system.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
      };
      Install.WantedBy = [ "sockets.target" ];
    };
  };
}
