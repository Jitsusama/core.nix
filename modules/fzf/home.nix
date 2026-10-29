# fzf, the fuzzy finder behind the shell's history and file searches, drawn
# like the launcher: the current row in the selection colour and the
# strongest text, the rest in text and muted, and the accent where the
# keys go. Its shell bindings are written when the account is built rather
# than asked of fzf at each shell start, in the place home-manager would put
# them, after zsh's own keys, so its Ctrl+R is the one kept.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) colours;
  bindings = pkgs.runCommandLocal "fzf.zsh" { } ''
    ${lib.getExe config.programs.fzf.package} --zsh > $out
  '';
in
{
  imports = [ ../theme/home.nix ];

  programs.fzf = {
    enable = true;
    enableZshIntegration = false;
    defaultOptions = [
      "--height=40%"
      "--layout=reverse"
      "--info=inline"
      "--border"
      # The current row is lit across, as a launcher row is.
      "--highlight-line"
    ];
    colors = {
      fg = colours.text;
      bg = "-1";
      hl = colours.strong;
      "fg+" = colours.strong;
      "bg+" = colours.selection;
      "hl+" = colours.strong;
      gutter = colours.background;
      query = colours.text;
      prompt = colours.accent;
      pointer = colours.accent;
      marker = colours.accent;
      info = colours.muted;
      spinner = colours.muted;
      header = colours.muted;
      label = colours.muted;
      border = colours.rule;
      separator = colours.rule;
      scrollbar = colours.rule;
    };
  };

  programs.zsh.initContent = lib.mkOrder 910 ''
    [[ $options[zle] = on ]] && source ${bindings}
  '';
}
