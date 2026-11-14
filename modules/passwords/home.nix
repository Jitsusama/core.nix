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
# entry without its secrets, and answers the shell alone: the Passwords
# place runs it as its own child and asks it over its standard input and
# output, in the shell's daemon interface. There's no unit and no socket,
# since every agent runs as Joel, and a daemon any of them could reach
# would answer them with whatever 1Password let it read (2026-11-10, Joel:
# "we can't have a password backdoor here"); 1Password lets in each
# process tree apart, so a daemon an agent starts must ask Joel itself. A
# secret goes from its manager only to the ask for it; nothing is copied
# while no copier is named, which nothing names until agents are fenced
# off the clipboard. An audit, only when Joel asks, reads every password
# to find those reused and those breached, hashing each in the daemon's
# memory and sending Have I Been Pwned the first five characters of a
# hash alone. What the last check found, each troubled entry's two counts
# and when trouble was found and cleared, is kept in the account's state
# folder, so For You's to-do for it outlives a reboot; never a hash or a
# password.
#
# Secret asks are answered by the same daemon, for the references
# jitsusama.passwords.releases lists alone, each read by the manager whose
# `references` prefix it starts with: an agent's ask is a row among
# Passwords' own, and only Joel's yes on the shell's channel to its child
# releases it, into one command run in bubblewrap (src/asks.rs). None
# listed, the default, is off: no folder is watched and no row drawn.
#
# `passwords` is left with `keys`, the public keys 1Password's SSH agent
# holds, and completes in zsh with the completion it writes itself
# (completion.nix). No command reaches the daemon.
{ quickshell }:
{
  config,
  lib,
  options,
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
      managers = map (
        manager:
        {
          inherit (manager) name command called;
          shows = if manager.shows == null then "" else manager.shows;
        }
        // lib.optionalAttrs (manager.references != null) { inherit (manager) references; }
      ) managers;
      # What a secret ask may release, and the programs asks are answered
      # with: git reads an ask's tree, bubblewrap runs its command, and
      # loginctl says whether the session is locked, when no yes counts.
      inherit (config.jitsusama.passwords) releases;
      git = lib.getExe pkgs.git;
      bwrap = lib.getExe pkgs.bubblewrap;
      loginctl = "${pkgs.systemd}/bin/loginctl";
    }
  );
  # The launcher's vocabulary, which the daemon says its rows in.
  words = pkgs.writeText "words.json" (builtins.toJSON config.jitsusama.quickshell.launcher.words);
in
{
  imports = [ quickshell ];

  options.jitsusama.passwords.releases = lib.mkOption {
    type = lib.types.listOf lib.types.str;
    default = [ ];
    example = [ "op://Private/Release Test/password" ];
    description = ''
      The references a secret ask may have released, each matched whole,
      and read by the manager whose references start as it does. None, the
      default, turns asks off: the daemon watches no folder, and the
      launcher draws no ask. It holds references, never values.
    '';
  };

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
          references = lib.mkOption {
            type = lib.types.nullOr lib.types.str;
            default = null;
            example = "op://";
            description = ''
              What the references it releases to a secret ask start with, or
              none for a manager that releases nothing. It's asked `release`
              with the `reference` and `asked`, and answers its `value`.
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
    # The experiment watches the same asks folder, and would answer what
    # this daemon serves, so the two never run together.
    assertions = [
      {
        assertion = config.jitsusama.passwords.releases == [ ] || !(options.jitsusama ? secret-asks);
        message = ''
          jitsusama.passwords.releases turns on the passwords daemon's own
          secret asks, so the secret-asks experiment, which watches the same
          folder, can't be imported beside it.
        '';
      }
    ];

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
      # Off, reaching no daemon, until agents are fenced off Joel's runtime
      # folder and Wayland; the checks that walk it turn it on, where no
      # agent runs and no secret is real.
      off = true;
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
      # The daemon the place runs as its own child, for the shell's life.
      daemon = [
        (lib.getExe passwords)
        "daemon"
        "${managersFile}"
        "${words}"
      ];
    };
  };
}
