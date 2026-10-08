{
  programs.broot = {
    enable = true;
    enableZshIntegration = true;
    settings = {
      modal = false;
      show_selection_mark = false;
      terminal_title = "📁 {git-name}";
      icon_theme = "nerdfont";
    };
  };
}
