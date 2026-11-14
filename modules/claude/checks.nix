# The checks of Claude Code's module, which checks.nix gathers with every
# other folder's; its opening comment says what each kind of check proves.
{
  account,
  holds,
  lib,
  macWorkstation,
  passesClippy,
  pkgs,
  workstation,
  ...
}:
let
  linuxAccount = account workstation;
  linux = linuxAccount.programs.claude-code;
  mac = (account macWorkstation).programs.claude-code;

  # The theme's strong colour as a terminal is told it, red;green;blue, which
  # the folder is drawn in.
  inherit (linuxAccount.jitsusama.theme.colours) strong;
  channel = at: toString (lib.fromHexString (builtins.substring at 2 strong));
  strongDrawn = lib.concatMapStringsSep ";" channel [
    1
    3
    5
  ];

  # The status line, asked for ink, since Claude Code reads it through a pipe.
  inked = ".*/bin/claude-statusline --color always";
  runsInked = command: builtins.match inked command != null;
in
{
  # Both kinds of workstation show Claude Code the status line, in settings
  # it can still write.
  claude-shows-its-status-line =
    holds "claude-shows-its-status-line"
      "Claude Code isn't told of the status line, or its settings would be read-only."
      (
        lib.all
          (
            claude:
            claude.mutableSettings
            && claude.settings.statusLine.type == "command"
            && runsInked claude.settings.statusLine.command
          )
          [
            linux
            mac
          ]
      );

  # The status line's tests run as it builds (statusline/), and the command a
  # workstation gives Claude Code draws a session's folder in the theme's
  # strong colour, read from the account's theme as kitty's environment has it.
  claude-status-line-says-how-it-stands =
    pkgs.runCommandLocal "claude-status-line-says-how-it-stands" { }
      ''
        mkdir -p config/jitsusama
        cp ${linuxAccount.xdg.configFile."jitsusama/theme.json".source} config/jitsusama/theme.json
        echo '{"workspace": {"current_dir": "/src/core.nix"}}' \
          | XDG_CONFIG_HOME=$PWD/config COLORTERM=truecolor TERM=xterm-kitty \
            ${linux.settings.statusLine.command} > line
        grep -qF "$(printf '\033[1;38;2;${strongDrawn}m')core.nix" line
        touch $out
      '';

  # clippy finds nothing in the status line, its tests included.
  claude-statusline-passes-clippy = passesClippy "claude-statusline-passes-clippy" (
    pkgs.callPackage ./statusline/package.nix { }
  );

  # The status line's tests run as it builds: the context its transcript
  # holds, rounded as it reads, ink only where asked, and a session it
  # can't read named.
  claude-statusline-says-how-full-the-context-is = pkgs.callPackage ./statusline/package.nix { };
}
