# Passwords: what Joel's password managers keep, in the launcher's
# Passwords place and on the command line, as People is for everyone he
# deals with. Each password manager is a program on Passwords' line
# protocol, named in jitsusama.passwords.managers by the module that
# brings the manager, as onepassword's does; this names none, so a manager
# plugs in from anywhere, a machine's own repository included, with no
# change here. A module with a manager imports this one, so Passwords is
# there only where some manager is.
#
# The passwords daemon (src/main.rs) runs each manager, holding every
# entry without its secrets over $XDG_RUNTIME_DIR/daemons/passwords.sock in
# the shell's daemon interface, for the shell and for `passwords`, the
# command that asks it. A secret goes from its manager only to whoever
# asked to see it, or to the clipboard, marked sensitive and cleared after
# 90 seconds; nothing here or there keeps one.
{ quickshell }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  # The managers in the order they're consulted, lowest first, which is
  # the order their entries are listed in.
  managers = lib.sortOn (manager: manager.order) (
    lib.mapAttrsToList (name: manager: manager // { inherit name; }) config.jitsusama.passwords.managers
  );
  passwords = pkgs.callPackage ./package.nix { };
  managersFile = pkgs.writeText "password-managers.json" (
    builtins.toJSON {
      managers = map (manager: { inherit (manager) name command; }) managers;
    }
  );
in
{
  imports = [ quickshell ];

  options.jitsusama.passwords.managers = lib.mkOption {
    type = lib.types.attrsOf (
      lib.types.submodule {
        options = {
          order = lib.mkOption {
            type = lib.types.int;
            description = "Where it's consulted, lowest first: its entries are listed in this order.";
          };
          command = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            description = ''
              The program and its arguments the passwords daemon runs for it,
              speaking Passwords' line protocol: a JSON line in for each ask
              (`list`, `get`, `reveal`, `code` or `let-in`, with its `id`),
              and a JSON line out answering it by that id, or saying how it
              stands (`state` and `reason`) unasked. It never gives a hidden
              detail's value but to `reveal`.
            '';
          };
          program = lib.mkOption {
            type = lib.types.str;
            description = ''
              The desktop entry its program row in the launcher is made from,
              which holds what the manager gives it, such as Let the Launcher
              In while it can't read.
            '';
          };
          called = lib.mkOption {
            type = lib.types.str;
            description = "Its name as Joel reads it.";
          };
        };
      }
    );
    default = { };
    description = ''
      The password managers, by name, each with its order, the program the
      daemon runs for it, its desktop entry and its name.
    '';
  };

  config = lib.mkIf (managers != [ ]) {
    home.packages = [ passwords ];

    # After Slack's program, before Search Slack, among the programs' own.
    jitsusama.quickshell.launcher.providers.passwords = {
      order = 48;
      source = ./Passwords.qml;
      managers = map (manager: { inherit (manager) name program called; }) managers;
    };

    # The passwords daemon. It isn't in background.slice: what it holds is
    # what the launcher shows as Joel steps into Passwords, so it answers
    # ahead of the work behind.
    systemd.user.services.passwords = {
      Unit = {
        Description = "Holds Joel's passwords ready for the shell, without their secrets";
        # Restarted after a switch reloads the units, it comes back with the
        # managers the switch named, as the new one.
        X-SwitchMethod = "restart";
        # Its socket is systemd's, handed to it as it starts. It runs with
        # the graphical session, whose display wl-copy copies to and where
        # a password manager's own app, as 1Password's, answers.
        Requires = [ "passwords.socket" ];
        After = [
          "passwords.socket"
          "graphical-session.target"
        ];
        PartOf = [ "graphical-session.target" ];
      };
      Service = {
        ExecStart = "${lib.getExe passwords} daemon %t/daemons/passwords.sock ${managersFile}";
        # wl-copy holds a copied secret on the clipboard until it's cleared.
        Environment = [ "PASSWORDS_COPIER=${lib.getExe' pkgs.wl-clipboard "wl-copy"}" ];
        Restart = "on-failure";
        RestartSec = 5;
        # Nothing it holds is ever written, and a core dump would write it.
        LimitCORE = 0;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };

    # Its socket, held by systemd whether or not the daemon runs, so the
    # shell asking while it starts waits on it rather than finding no
    # socket; asking starts it, should it have stopped.
    systemd.user.sockets.passwords = {
      Unit.Description = "Where the shell asks the passwords daemon";
      Socket = {
        ListenStream = "%t/daemons/passwords.sock";
        SocketMode = "0600";
        DirectoryMode = "0700";
      };
      Install.WantedBy = [ "sockets.target" ];
    };
  };
}
