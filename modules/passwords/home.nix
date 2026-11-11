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
# 90 seconds; nothing here or there keeps one. An audit, only when Joel
# asks, reads every password to find those reused and those breached,
# hashing each in the daemon's memory and sending Have I Been Pwned the
# first five characters of a hash alone; the unit restricts no network, so
# that reaches it as it is. What the last check found, each troubled
# entry's two counts and when trouble was found and cleared, is kept in
# the account's state folder, so For You's to-do for it outlives a reboot;
# never a hash or a password.
#
# `passwords` completes in zsh with the completion it writes itself
# (completion.nix), which offers the entries and details the daemon holds
# and never asks a manager anything, so a Tab can't make 1Password ask
# Joel to let it in. `passwords help` says each command, what it prints
# for a person and for a program, and its exit statuses.
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
      # Its name as Joel reads it and its own link to an entry, for the
      # rows the daemon serves the launcher (src/rows.rs).
      managers = map (manager: {
        inherit (manager) name command called;
        shows = if manager.shows == null then "" else manager.shows;
      }) managers;
    }
  );
  # The launcher's vocabulary, which the daemon says its rows in.
  words = pkgs.writeText "words.json" (builtins.toJSON config.jitsusama.quickshell.launcher.words);
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
              (`list`, `get`, `reveal`, `code`, `audit` or `let-in`, with its
              `id`), and a JSON line out answering it by that id, or saying
              how it stands (`state` and `reason`) unasked. It never gives a
              hidden detail's value but to `reveal`, and every password only
              to `audit`, asked by Joel alone.
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
          windows = lib.mkOption {
            type = lib.types.listOf lib.types.str;
            default = [ ];
            description = ''
              The app ids its program's windows carry, so one that opens as
              it asks Joel something, such as to unlock, closes the launcher
              and takes the keys rather than coming up beneath it.
            '';
          };
          shows = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            description = ''
              The address that shows an entry in the manager's own program,
              with `{account}`, `{collection}` and `{id}` where the entry's
              own account, its collection's id and its id go, or none for a
              manager whose program can't be shown one.
            '';
          };
          fills = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            description = ''
              The desktop entry of the browser whose extension of the
              manager's own fills a sign-in, which an entry's Open the Website
              opens its web page in, or none for a manager with no such
              extension. The extension offers to fill it there itself, so no
              secret passes through the launcher or the daemon.
            '';
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
    home.packages = [
      passwords
      (pkgs.callPackage ./completion.nix { })
    ];

    # What the place works out apart from drawing it, and what For You's
    # to-do for the last check works out, beside it, and the copy it last
    # made, which the place and the margin's mark both read.
    xdg.configFile = {
      "quickshell/launcher/Passwords.js".source = ./Passwords.js;
      "quickshell/launcher/Managers.js".source = ./Managers.js;
      "quickshell/launcher/Checked.js".source = ./Checked.js;
      "quickshell/Clearing.qml".source = ./Clearing.qml;
    };

    # A password or a one-time code on the clipboard until it clears, in
    # the margin wearing the clipboard's mark, since the launcher goes as
    # it copies; after what Joel left on himself, before what's on air.
    jitsusama.quickshell.margin.pieces.clearing = {
      order = 66;
      source = ./Copied.qml;
      label = "clearing";
      tier = "when";
      mark = "clipboard";
    };

    # After Slack's program, before Search Slack, among the programs' own.
    jitsusama.quickshell.launcher.providers.passwords = {
      order = 48;
      source = ./Passwords.qml;
      managers = map (manager: {
        inherit (manager)
          name
          program
          called
          windows
          ;
        shows = if manager.shows == null then "" else manager.shows;
        fills = if manager.fills == null then "" else manager.fills;
      }) managers;
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
        ExecStart = lib.concatStringsSep " " [
          (lib.getExe passwords)
          "daemon"
          "%t/daemons/passwords.sock"
          "${managersFile}"
          "${words}"
        ];
        # wl-copy holds a copied secret on the clipboard until it's cleared.
        Environment = [ "PASSWORDS_COPIER=${lib.getExe' pkgs.wl-clipboard "wl-copy"}" ];
        Restart = "on-failure";
        RestartSec = 5;
        # What the checks found is kept in ~/.local/state/passwords, made
        # by systemd for Joel alone; it's the one thing the daemon writes.
        StateDirectory = "passwords";
        StateDirectoryMode = "0700";
        # No secret it holds is ever written, and a core dump would write one.
        LimitCORE = 0;
      };
      # Nothing starts it, nor its socket below, until agents are fenced
      # off Joel's runtime folder and Wayland: every agent runs as his uid,
      # so any of them could ask it to reveal, copy or give a code once
      # 1Password lets it in (2026-11-10, Joel: "we can't have a password
      # backdoor here"). It runs only when started by hand, as the desktop's
      # VM tests do; the Passwords place says it can't reach it meanwhile.
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
    };
  };
}
