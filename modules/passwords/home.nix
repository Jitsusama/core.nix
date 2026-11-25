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
# Passwords' own, and only Joel's yes from the shell, its one client,
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
      # Whether it lays a throwaway layer over a folder (--tmp-overlay,
      # from 0.10), which the caches of tools that write in place take.
      overlays = lib.versionAtLeast pkgs.bubblewrap.version "0.10.0";
      loginctl = "${pkgs.systemd}/bin/loginctl";
      # The forge a yes may stand for: a clean tree whose origin is a
      # repository on it, at a commit its main holds, as GitHub says.
      forges = [ "https://github.com/" ];
      # The gh whose token GitHub is asked with, so a private repository's
      # main is said too: its store path, which no agent can repoint.
      gh = lib.getExe config.programs.gh.package;
      # The programs an ask may run, and its PATH: the system's and the
      # account's own, as home-manager puts them in /etc/profiles. Never
      # `~/.nix-profile`, a link the account owns, which an agent could
      # repoint (src/asks.rs's chosen_by_root).
      profiles = [
        "/run/current-system/sw/bin"
        "/etc/profiles/per-user/${config.home.username}/bin"
      ];
      # The agent fence's list of what it hides, which an ask's command is
      # kept from too, read from the harness's install at every ask so its
      # own update reaches it; where it isn't there, every ask is refused.
      hidden =
        "${config.xdg.dataHome}/pi-packages/github.com/Jitsusama/agentic-harness.pi"
        + "/current/data/fence-hidden.json";
    }
  );
  # The launcher's vocabulary, which the daemon says its rows in.
  words = pkgs.writeText "words.json" (builtins.toJSON config.jitsusama.quickshell.launcher.words);
  # Whether secret asks are answered and drawn: once a release is listed,
  # and only while the place is on, since an off place's daemon never runs.
  # Read only outside the place's own settings, which Nix merges by reading
  # each, so one reading the place's off would wait on itself.
  answersAsks =
    config.jitsusama.passwords.releases != [ ]
    && !(config.jitsusama.quickshell.launcher.providers.passwords.off or false);
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
    }
    # How long an ask's line was shown, which Asks reads beside it.
    // lib.optionalAttrs (config.jitsusama.passwords.releases != [ ]) {
      "quickshell/launcher/Shown.js".source = ../secret-asks/Shown.js;
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
      # On, its entries' Return typing a password into the window the
      # launcher opened over, with nothing copied while no copier is named,
      # as copying waits until agents are fenced off the clipboard. Only
      # Joel's keys in the launcher reach a type, and nothing is typed into
      # a pi session's window (src/typing.rs). A machine sets off to true
      # for the place to reach no daemon at all.
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
      # Its daemon answers secret asks once one is listed, and serves them
      # on the place's own connection, which Asks draws them over; an off
      # place runs no daemon, so it needn't say the place is on.
      answersAsks = config.jitsusama.passwords.releases != [ ];
      # What types a password into the window the launcher opened over, as
      # an entry's Return does while copying is off: wtype, reading it on
      # its standard input, never its arguments; and kitten, which asks a
      # kitty what runs in its window, so nothing is typed into a terminal
      # running a pi session (src/typing.rs).
      typist = lib.getExe pkgs.wtype;
      kitty = lib.getExe' pkgs.kitty "kitten";
      # The daemon the place runs as its own child, for the shell's life.
      daemon = [
        (lib.getExe passwords)
        "daemon"
        "${managersFile}"
        "${words}"
      ];
    };

    # Asks, inside Passwords, right after it, where any release is listed
    # and the place is on: each ask this daemon serves, drawn over the
    # place's own connection rather than a socket, and For You's thing to
    # do while one waits. Its enable, not its being, follows the place, so
    # reading the place's setting asks nothing of the providers' names.
    jitsusama.quickshell.launcher.providers.asks =
      lib.mkIf (config.jitsusama.passwords.releases != [ ])
        {
          enable = answersAsks;
          order = 49;
          source = ../secret-asks/Asks.qml;
          over = "passwords";
        };

    # A release listed while the place is off would wait on a daemon the
    # shell never runs, so the machine is told rather than left wondering.
    warnings = lib.optional (config.jitsusama.passwords.releases != [ ] && !answersAsks) ''
      jitsusama.passwords.releases lists releases, but the launcher's
      Passwords place is off, so no secret ask is answered or drawn until
      jitsusama.quickshell.launcher.providers.passwords.off is false.
    '';
  };
}
