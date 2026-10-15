# pi, the coding agent in the terminal: a theme drawn from the machine's, so
# its messages, tools, diffs and code wear the same colours as everything
# else. pi keeps which theme it uses in a settings file of its own, so this
# only puts the theme where pi finds it, for Joel to pick in /settings.
{ config, lib, ... }:
let
  inherit (config.jitsusama.theme) appearance roles syntax;
  mix = import ../theme/mix.nix { inherit lib; };

  # A tool's result is a faint tint of how it went, under text that stays
  # the text's own colour.
  tint = weight: colour: mix weight colour roles.background;

  # pi's own names for what it colours, in its theme format:
  # https://pi.dev/schemas/theme.schema.json
  theme = {
    name = "jitsusama";
    inherit appearance;
    colors = {
      inherit (roles)
        accent
        text
        success
        warning
        ;
      error = roles.alert;
      border = roles.rule;
      borderAccent = roles.accent;
      borderMuted = roles.edge;
      inherit (roles) muted;
      dim = roles.muted;
      thinkingText = roles.muted;
      selectedBg = roles.selection;
      searchMatchBg = roles.raised;
      searchMatchText = roles.strong;
      scrollbarTrack = roles.raised;
      scrollbarThumb = roles.rule;
      # What Joel wrote sits on the sunken shade, as a drawer's inside does.
      userMessageBg = roles.sunken;
      userMessageText = roles.text;
      customMessageBg = roles.sunken;
      customMessageText = roles.muted;
      customMessageLabel = roles.voice;
      toolPendingBg = roles.sunken;
      toolSuccessBg = tint 0.1 roles.success;
      toolErrorBg = tint 0.12 roles.alert;
      toolTitle = roles.strong;
      toolOutput = roles.muted;
      # Markdown reads as Neovim draws it: headings strong, code as strings,
      # the marks between muted.
      mdHeading = roles.strong;
      mdLink = roles.program;
      mdLinkUrl = roles.muted;
      mdCode = syntax.string.colour;
      mdCodeBlock = roles.text;
      mdCodeBlockBorder = roles.rule;
      mdQuote = roles.muted;
      mdQuoteBorder = roles.rule;
      mdHr = roles.rule;
      mdListBullet = syntax.punctuation.colour;
      toolDiffAdded = roles.success;
      toolDiffRemoved = roles.alert;
      toolDiffContext = roles.muted;
      syntaxComment = syntax.comment.colour;
      syntaxKeyword = syntax.keyword.colour;
      syntaxFunction = syntax.function.colour;
      syntaxVariable = roles.text;
      syntaxString = syntax.string.colour;
      syntaxNumber = syntax.constant.colour;
      syntaxType = syntax.type.colour;
      syntaxOperator = syntax.operator.colour;
      syntaxPunctuation = syntax.punctuation.colour;
      # How hard pi thinks warms from the muted text to the alert, a step
      # through the glyphs' hues at each level.
      thinkingOff = roles.muted;
      thinkingMinimal = syntax.type.colour;
      thinkingLow = roles.program;
      thinkingMedium = roles.voice;
      thinkingHigh = roles.person;
      thinkingXhigh = roles.warning;
      thinkingMax = roles.alert;
      # A shell command is something to do.
      bashMode = roles.action;
    };
    export = {
      pageBg = roles.background;
      cardBg = roles.sunken;
      infoBg = roles.raised;
    };
  };
in
{
  imports = [ ../theme/home.nix ];

  home.file.".pi/agent/themes/jitsusama.json".text = builtins.toJSON theme;
}
