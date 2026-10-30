# Slack's keys, for the launcher to list over its window, whose app id is
# slack, and Slack as one of People's sources. Slack comes from wherever
# the machine gets it, so this lists its keys and installs nothing of
# Slack's own; a machine with Slack imports it. Slack takes no settings
# for its keys, so these are its own, from its help's list for Windows and
# Linux.
{ people }:
{ config, pkgs, ... }:
let
  grouped = group: map (key: key // { inherit group; });
  # Who Slack knows by an address, only ever read (slack-people).
  lookUp = pkgs.callPackage ./package.nix { };
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
    credentials = "${config.xdg.configHome}/agentic-harness/slack/credentials.json";
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
