# Slack's keys, for the launcher to list over its window, whose app id is
# slack, and Slack as one of People's sources. Slack comes from wherever
# the machine gets it, so this lists its keys and installs nothing of
# Slack's own; a machine with Slack imports it. Slack takes no settings
# for its keys, so these are its own, from its help's list for Windows and
# Linux.
{ people }:
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
in
{
  imports = [
    ../keys/home.nix
    people
  ];

  home.packages = [ lookUp ];

  # What the source decides, beside it, where its import finds it.
  xdg.configFile."quickshell/launcher/people/Slack.js".source = ./Slack.js;

  # Write in Slack for anyone Slack knows by one of their addresses, through
  # the harness's Slack sign-in, read only (Joel, 2026-10-28). It's the
  # first source asked, since a message is the lightest way to reach
  # someone, before a call or a mail (VKCF3Z, 3bbad443). With no sign-in,
  # nothing is asked.
  jitsusama.people.sources.slack = {
    order = 5;
    source = ./SlackSource.qml;
    inherit credentials;
    # The same program, run by the people daemon, which asks it only what
    # a client of the daemon asks.
    daemon = [
      (lib.getExe lookUp)
      credentials
    ];
  };

  # What in Slack wants Joel, kept in the runtime folder's
  # slack-unreads.json for Now's Slack piece: direct messages with
  # unreads, mentions of him and his mentions in threads, read only, and
  # started once pi has signed in to Slack.
  systemd.user.paths.slack-unreads = {
    Unit.Description = "Starts keeping Slack's unreads once pi has signed in";
    Path.PathExists = credentials;
    Install.WantedBy = [ "default.target" ];
  };

  systemd.user.services.slack-unreads = {
    Unit = {
      Description = "Keeps what in Slack wants Joel for the shell";
      # As google's does: restarted after a switch reloads the units, it
      # comes back as the new one.
      X-SwitchMethod = "restart";
    };
    Service = {
      Slice = "background.slice";
      ExecStart = lib.concatStringsSep " " [
        "${lookUp}/bin/slack-unreads"
        (lib.escapeShellArg credentials)
        "%t/slack-unreads.json"
      ];
      Restart = "on-failure";
      RestartSec = 30;
      # It tells the watchdog after each ask, half a minute apart, so one
      # that hangs is killed and started again.
      WatchdogSec = 180;
      NotifyAccess = "main";
      # What it counted goes with it, however it stops, so the shell never
      # shows a count nothing is keeping current.
      ExecStopPost = lib.concatStringsSep " " [
        "${pkgs.coreutils}/bin/rm -f"
        "%t/slack-unreads.json"
        "%t/slack-unreads.new"
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
