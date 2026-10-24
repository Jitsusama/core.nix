# Claude Code, Anthropic's coding agent in the terminal: the program, its
# status line, and a theme drawn from the machine's, so its text, its diffs
# and its prompts wear the same colours as everything else.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) appearance roles terminal;
  mix = import ../theme/mix.nix { inherit lib; };

  # The status line (statusline/), and each part's colour in the theme for
  # what it is: the folder strong, the model a program's, the context's
  # size a success, and the separators the rules'.
  statusline = pkgs.callPackage ./statusline/package.nix { };
  statuslineColours = builtins.toFile "claude-statusline-colours.json" (
    builtins.toJSON {
      folder = roles.strong;
      model = roles.program;
      context = roles.success;
      separator = roles.rule;
    }
  );

  # A diff's lines and words are tinted backgrounds under the text, so a
  # change reads at a glance without the text itself changing colour. The
  # alert is the brighter of the two, so it takes less to weigh the same.
  tint = weight: colour: mix weight colour roles.background;

  # Claude Code's own names for what it colours, in its custom theme format.
  # The subagents' and the rainbow's take the terminal's. The base isn't its
  # ANSI one, which would draw code in the terminal's colours too but greys
  # out a diff's lines, so code keeps Claude Code's own Monokai.
  theme = {
    name = "Jitsusama";
    base = appearance;
    overrides = {
      claude = roles.program;
      claudeShimmer = roles.strong;
      clawd_body = roles.program;
      clawd_background = roles.deepest;
      inherit (roles) text;
      inverseText = roles.deepest;
      inactive = roles.muted;
      inactiveShimmer = roles.text;
      subtle = roles.muted;
      promptBorder = roles.rule;
      promptBorderShimmer = roles.muted;
      suggestion = roles.accent;
      remember = roles.accent;
      permission = roles.action;
      permissionShimmer = roles.strong;
      planMode = roles.voice;
      ide = roles.program;
      background = roles.voice;
      inherit (roles) success;
      error = roles.alert;
      inherit (roles) warning;
      warningShimmer = roles.strong;
      merged = roles.person;
      diffAdded = tint 0.25 roles.success;
      diffRemoved = tint 0.18 roles.alert;
      diffAddedDimmed = tint 0.12 roles.success;
      diffRemovedDimmed = tint 0.09 roles.alert;
      diffAddedWord = tint 0.45 roles.success;
      diffRemovedWord = tint 0.35 roles.alert;
      userMessageBackground = roles.raised;
      userMessageBackgroundHover = roles.selection;
      composerSidebarBackground = roles.raised;
      bashMessageBackgroundColor = roles.sunken;
      memoryBackgroundColor = roles.raised;
      selectionBg = roles.selection;
      rate_limit_fill = roles.warning;
      rate_limit_empty = roles.raised;
      red_FOR_SUBAGENTS_ONLY = terminal.red;
      blue_FOR_SUBAGENTS_ONLY = terminal.blue;
      green_FOR_SUBAGENTS_ONLY = terminal.green;
      yellow_FOR_SUBAGENTS_ONLY = terminal.bright_yellow;
      purple_FOR_SUBAGENTS_ONLY = terminal.magenta;
      orange_FOR_SUBAGENTS_ONLY = terminal.bright_red;
      pink_FOR_SUBAGENTS_ONLY = terminal.bright_magenta;
      cyan_FOR_SUBAGENTS_ONLY = terminal.cyan;
      rainbow_red = terminal.red;
      rainbow_orange = terminal.bright_red;
      rainbow_yellow = terminal.bright_yellow;
      rainbow_green = terminal.green;
      rainbow_blue = terminal.cyan;
      rainbow_indigo = terminal.blue;
      rainbow_violet = terminal.magenta;
    };
  };
in
{
  imports = [ ../theme/home.nix ];

  programs.claude-code = {
    enable = true;
    # Claude Code writes its settings file as it runs (permissions, /model,
    # onboarding, sandbox setup), so each switch merges what's declared into
    # a writable file rather than linking a read-only one, and what Claude
    # Code saved stays.
    mutableSettings = true;
    settings.statusLine = {
      type = "command";
      command = "${lib.getExe statusline} ${statuslineColours}";
    };
  };
  programs.git.ignores = [ "CLAUDE.local.md" ];

  # Claude Code keeps its choice of theme in ~/.claude.json, a file it
  # rewrites as it runs, so the theme is offered rather than chosen: /theme
  # lists it as Jitsusama, and it follows the machine's theme from then on.
  home.file.".claude/themes/jitsusama.json".text = builtins.toJSON theme;
}
