# The release line's first experiment, for Joel to try before the real
# one: an agent's ask for a secret is a file in $XDG_RUNTIME_DIR/secret-asks,
# the secret-asks daemon, a program of core's own (src/daemon.rs), serves
# each as a row in the launcher's Asks over
# $XDG_RUNTIME_DIR/daemons/secret-asks.sock, and Joel's yes there reads one
# item from 1Password into one fixed checker, which says only how long it
# was. An ask for any other item, or any other command, is refused unseen,
# so the item named here must be a throwaway one made for the experiment:
# what this doesn't hold yet (daemon.rs) is that only his key says yes.
#
# Importing it chooses it; the item is the machine's to name. The shell
# runs only on Linux.
{
  config,
  lib,
  osConfig ? { },
  pkgs,
  ...
}:
let
  package = pkgs.callPackage ./package.nix { };
  settings = config.jitsusama.secret-asks;
in
{
  options.jitsusama.secret-asks = {
    item = lib.mkOption {
      type = lib.types.strMatching "op://[^/]+/[^/]+/[^/]+";
      example = "op://Private/Release Test/password";
      description = ''
        The one 1Password item, as op names it, that the experiment may
        release: a throwaway one made for it, since a program running as
        Joel could say yes for it.
      '';
    };
    account = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "my.1password.com";
      description = "1Password's account holding the item, where op's default isn't it.";
    };
  };

  config = lib.mkIf pkgs.stdenv.hostPlatform.isLinux {
    systemd.user.services.secret-asks = {
      Unit = {
        Description = "Asks Joel in the launcher before releasing the experiment's item";
        X-SwitchMethod = "restart";
        # Its socket is systemd's, handed to it as it starts.
        Requires = [ "secret-asks.socket" ];
        After = [ "secret-asks.socket" ];
      };
      Service = {
        # Started as @<program> secret-asks, so ps reads its name rather than a
        # store path, as docs/daemons.md says every daemon is.
        ExecStart = lib.escapeShellArgs (
          [
            "@${lib.getExe package}"
            "secret-asks"
            "serve"
            "--socket"
            "%t/daemons/secret-asks.sock"
            "--asks"
            "%t/secret-asks"
            "--vocabulary"
            "${(import ../quickshell/launcher/words.nix).file}"
            "--item"
            settings.item
            "--op"
            "${osConfig.security.wrapperDir or "/run/wrappers/bin"}/op"
            "--git"
            (lib.getExe pkgs.git)
            "--loginctl"
            "${pkgs.systemd}/bin/loginctl"
          ]
          ++ lib.optionals (settings.account != null) [
            "--account"
            settings.account
          ]
        );
        Restart = "always";
        RestartSec = 5;
        Slice = "background.slice";
      };
      Install.WantedBy = [ "default.target" ];
    };

    # Its socket, held by systemd whether or not the daemon runs, so the
    # shell asking while it starts waits on it rather than finding none.
    systemd.user.sockets.secret-asks = {
      Unit.Description = "Where the launcher asks for Asks' rows";
      Socket = {
        ListenStream = "%t/daemons/secret-asks.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
      };
      Install.WantedBy = [ "sockets.target" ];
    };

    # Puts up an ask, as an agent would, for trying the line by hand.
    home.packages = [ package ];
  };
}
