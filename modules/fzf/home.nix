# fzf, the fuzzy finder behind the shell's history and file searches, drawn
# like the launcher: the current row in the selection colour and the
# strongest text, the rest in text and muted, and the accent where the
# keys go.
{ config, ... }:
let
  inherit (config.jitsusama.theme) roles;
in
{
  imports = [ ../theme/home.nix ];

  programs.fzf = {
    enable = true;
    enableZshIntegration = true;
    defaultOptions = [
      "--height=40%"
      "--layout=reverse"
      "--info=inline"
      "--border"
      # The current row is lit across, as a launcher row is.
      "--highlight-line"
    ];
    colors = {
      fg = roles.text;
      bg = "-1";
      hl = roles.strong;
      "fg+" = roles.strong;
      "bg+" = roles.selection;
      "hl+" = roles.strong;
      gutter = roles.background;
      query = roles.text;
      prompt = roles.accent;
      pointer = roles.accent;
      marker = roles.accent;
      info = roles.muted;
      spinner = roles.muted;
      header = roles.muted;
      label = roles.muted;
      border = roles.rule;
      separator = roles.rule;
      scrollbar = roles.rule;
    };
  };
}
