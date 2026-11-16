# system says how the whole system stands at once: the machines of the
# realm, the mycelium, the home network and the code (src/lib.rs says how).
# It asks each part's own command, mycelium, net and repos, as they're found
# on the account's path, so it keeps no copy of theirs, and a machine
# without one of them says that part wasn't read.
#
# It's also a daemon, holding the record ready in the shell's daemon
# interface (src/serve.rs says how): on Linux over
# $XDG_RUNTIME_DIR/daemons/system.sock, for the system page and the
# launcher's home rows, and on a Mac, which has no runtime folder, in the
# account's state folder. Either way the machine's mycelium node reads it
# there and carries how the machine stands to every machine of the realm
# (../mycelium, healthSocket).
{
  config,
  lib,
  pkgs,
  ...
}:
let
  system = pkgs.callPackage ./package.nix { };
  inherit (pkgs.stdenv.hostPlatform) isDarwin;
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

  # A Mac has no systemd to hold its socket, so it listens on its own.
  launchd.agents.system = lib.mkIf isDarwin {
    enable = true;
    config = {
      ProgramArguments = [
        (lib.getExe system)
        "serve"
        "--socket"
        "${config.xdg.stateHome}/daemons/system.sock"
      ];
      # The owners it asks are the account's own commands, found where its
      # login shell finds them, as on Linux; without them every part reads
      # "not read".
      EnvironmentVariables.PATH = lib.concatStringsSep ":" [
        "${config.home.profileDirectory}/bin"
        "/run/current-system/sw/bin"
        "/usr/bin"
        "/bin"
      ];
      RunAtLoad = true;
      # As on Linux, it only stops on a fault; launchd waits 10 s between
      # starts.
      KeepAlive = true;
      # Whatever the account runs comes first, as background.slice does on
      # Linux.
      ProcessType = "Background";
      StandardErrorPath = "${config.home.homeDirectory}/Library/Logs/system.log";
    };
  };
}
