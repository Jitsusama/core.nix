# Slack's keys, for the launcher to list over its window, whose app id is
# slack, Slack as one of People's sources, and what in Slack wants Joel as
# one of what's on in Now. Slack comes from wherever
# the machine gets it, so this lists its keys and installs nothing of
# Slack's own; a machine with Slack imports it. Slack takes no settings
# for its keys, so these are its own, from its help's list for Windows and
# Linux.
{ people, quickshell }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  grouped = group: map (key: key // { inherit group; });
  # Who Slack knows by an address, only ever read (slack-people), and
  # what in Slack wants Joel (slack-unreads).
  lookUp = pkgs.callPackage ./package.nix { };
  credentials = "${config.xdg.configHome}/agentic-harness/slack/credentials.json";
  # Where Slack's ask of Joel waits, beside each quest's in For You
  # (WaitingOnYou.qml), named so no quest's address is it.
  waiting = "${config.xdg.stateHome}/mission/waiting/slack.json";
  # The launcher's vocabulary, which the rows slack-unreads serves are
  # said in (src/rows.rs), and slack-people's facts of someone
  # (src/person.rs).
  words = pkgs.writeText "words.json" (builtins.toJSON config.jitsusama.quickshell.launcher.words);
in
{
  imports = [
    ../keys/home.nix
    people
    quickshell
  ];

  xdg.configFile = {
    # A Slack conversation's one row, which every surface showing one
    # takes (SlackConversation.qml), a message's, wherever it's listed
    # (SlackMessage.qml), what in Slack wants him, watched on Slack's
    # daemon (SlackUnreads.qml), and For You's rows of it, as the daemon
    # serves them (SlackRows.qml).
    "quickshell/SlackConversation.qml".source = ./SlackConversation.qml;
    "quickshell/SlackMessage.qml".source = ./SlackMessage.qml;
    "quickshell/SlackMessage.js".source = ./SlackMessage.js;
    "quickshell/SlackRows.qml".source = ./SlackRows.qml;
    "quickshell/SlackUnreads.qml".source = ./SlackUnreads.qml;
    "quickshell/SlackUnreads.js".source = ./SlackUnreads.js;
  };

  # Slack's daemon tells a busy channel's post from a reply in a thread
  # Joel follows, which its notices say alike, over its socket below.
  jitsusama.quickshell.notices.senders.Slack.posted = "daemons/slack-unreads.sock";

  # What in Slack wants Joel, wearing the chat mark; right after the mail,
  # as VKCF3Z placed it, since both keep.
  jitsusama.quickshell.margin.pieces.slack = {
    order = 62;
    source = ./NewSlack.qml;
    label = "slack";
    tier = "when";
    mark = "chat";
  };

  # Why Slack can't be checked, said on its program row rather than in
  # Now, with Sign In first while it's signed out; after Tuple's and
  # GIMP's, beside the desktop's actions.
  jitsusama.quickshell.launcher.providers.slack-program = {
    order = 47;
    source = ./SlackProgram.qml;
  };

  # Write in Slack for anyone Slack knows by one of their addresses, through
  # the harness's Slack sign-in, read only (Joel, 2026-10-28). It's the
  # first source asked, since a message is the lightest way to reach
  # someone, before a call or a mail (VKCF3Z, 3bbad443). With no sign-in,
  # nothing is asked. The people daemon serves Slack's part of a person
  # whole, its act, conversation and facts ready, so it has no QML source.
  jitsusama.people.sources.slack = {
    order = 5;
    # Slack's reader, run by the people daemon alone, which asks it only
    # what a client of the daemon asks, or ahead of those it lists. It's
    # handed the vocabulary through env, which becomes slack-people, since
    # the daemon runs a source's command as it's given.
    daemon = [
      "${pkgs.coreutils}/bin/env"
      "LAUNCHER_WORDS=${words}"
      (lib.getExe lookUp)
      credentials
    ];
    # How someone stands in Slack, worded by the daemon for each entry it
    # serves and for those it lists in order: which of People's words (words.json) each
    # presence is, a presence People doesn't know said as it is, and their
    # status's emoji after it.
    stands = {
      field = "presence";
      words = {
        active = {
          word = "active";
          order = 0;
        };
        "in a huddle" = {
          word = "huddle";
          order = 1;
        };
        away = {
          word = "away";
          order = 2;
        };
      };
      others = true;
      mark = "emoji";
    };
  };

  # Search Slack for ‹words›, the act ending every search while Slack is
  # signed in, asked of Slack's daemon once Joel steps in (SlackSearch.qml).
  jitsusama.quickshell.launcher.providers.slack-search = {
    order = 51;
    source = ./SlackSearch.qml;
  };

  # What in Slack is unread, held by Slack's daemon for Now's Slack piece
  # and Slack's Unread: direct messages with unreads, mentions of him, his
  # mentions in threads and channels merely unread; and the conversations
  # he read last, for Slack's Recent. Read only, and started once pi has
  # signed in to Slack. While Slack refuses that sign-in, signing in again
  # waits in For You, in the file of what waits on Joel that's Slack's
  # own beside each quest's.
  systemd.user.paths.slack-unreads = {
    Unit.Description = "Starts keeping Slack's unreads once pi has signed in";
    Path.PathExists = credentials;
    Install.WantedBy = [ "default.target" ];
  };

  # Slack's daemon's socket, held by systemd whether or not it runs, so the
  # shell asking while it starts, or starts again, waits on it rather than
  # finding no socket and trying again blind (Quickshell warns of each,
  # every 2 s). Only once pi has signed in, as the daemon itself: with no
  # sign-in nothing listens, so asking starts nothing.
  systemd.user.sockets.slack-unreads = {
    Unit = {
      Description = "Where the shell asks Slack's daemon";
      ConditionPathExists = credentials;
    };
    Socket = {
      ListenStream = "%t/daemons/slack-unreads.sock";
      SocketMode = "0600";
      DirectoryMode = "0700";
    };
    Install.WantedBy = [ "sockets.target" ];
  };

  systemd.user.services.slack-unreads = {
    Unit = {
      Description = "Keeps what in Slack wants Joel for the shell";
      # As google's does: restarted after a switch reloads the units, it
      # comes back as the new one.
      X-SwitchMethod = "restart";
      # Its socket is systemd's, handed to it as it starts.
      Requires = [ "slack-unreads.socket" ];
      After = [ "slack-unreads.socket" ];
    };
    Service = {
      Slice = "background.slice";
      # Shown in ps by its name, not its store path (docs/daemons.md).
      ExecStart = lib.concatStringsSep " " [
        "@${lookUp}/bin/slack-unreads slack-unreads"
        (lib.escapeShellArg credentials)
        # Slack's daemon, where the shell watches what's unread and
        # searches Slack (src/daemon.rs).
        "%t/daemons/slack-unreads.sock"
        (lib.escapeShellArg waiting)
      ];
      # The vocabulary by its store path, read once as it starts.
      Environment = "LAUNCHER_WORDS=${words}";
      Restart = "on-failure";
      RestartSec = 30;
      # It tells the watchdog after each ask, half a minute apart, so one
      # that hangs is killed and started again.
      WatchdogSec = 180;
      NotifyAccess = "main";
      # What it asked of Joel goes with it, however it stops, so For You
      # never shows an ask nothing is keeping current; what it counted
      # goes from the shell as its socket stops answering.
      ExecStopPost = lib.concatStringsSep " " [
        "${pkgs.coreutils}/bin/rm -f"
        (lib.escapeShellArg waiting)
        (lib.escapeShellArg "${dirOf waiting}/.writing/${baseNameOf waiting}")
      ];
    };
  };

  jitsusama.keys.slack =
    grouped "conversations" [
      {
        chord = "Ctrl+k";
        does = "Jump to a conversation";
      }
      {
        chord = "Alt+Up";
        does = "The conversation above";
      }
      {
        chord = "Alt+Down";
        does = "The conversation below";
      }
      {
        chord = "Alt+Shift+Up";
        does = "The unread conversation above";
      }
      {
        chord = "Alt+Shift+Down";
        does = "The unread conversation below";
      }
      {
        chord = "Alt+Left";
        does = "Back in Slack";
      }
      {
        chord = "Alt+Right";
        does = "Forward in Slack";
      }
      {
        chord = "Ctrl+Shift+a";
        does = "Every unread message";
      }
      {
        chord = "Ctrl+Shift+t";
        does = "The threads";
      }
      {
        chord = "Ctrl+Shift+m";
        does = "The activity";
      }
      {
        chord = "Ctrl+Shift+2";
        does = "Browse direct messages";
      }
      {
        chord = "Ctrl+Shift+l";
        does = "Browse channels";
      }
    ]
    ++ grouped "messages" [
      {
        chord = "Ctrl+n";
        does = "Write a new message";
      }
      {
        chord = "Ctrl+o";
        does = "Upload a file";
      }
      {
        chord = "Ctrl+j";
        does = "The newest unread message";
      }
      {
        chord = "Escape";
        does = "Mark the conversation read";
      }
      {
        chord = "Shift+Escape";
        does = "Mark everything read";
      }
      {
        chord = "Ctrl+g";
        does = "Search Slack";
      }
      {
        chord = "Ctrl+f";
        does = "Search the conversation";
      }
    ]
    ++ [
      {
        chord = "Ctrl+Shift+h";
        does = "Start, join or leave a huddle";
      }
      {
        chord = "Ctrl+Shift+Space";
        does = "Mute or unmute the huddle";
      }
      {
        chord = "Ctrl+Shift+y";
        does = "Set your status";
      }
      {
        chord = "Ctrl+Shift+i";
        does = "The conversation's details";
      }
      {
        chord = "Ctrl+.";
        does = "Hide the right side";
      }
      {
        chord = "Ctrl+/";
        does = "Slack's own list of keys";
      }
    ];
}
