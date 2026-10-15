# zsh in each account: oh-my-zsh's plugins and the terminal's marks for each
# command. The prompt is its own module (modules/prompt).
{
  programs.zsh = {
    enable = true;
    sessionVariables = {
      LANG = "en_US.UTF-8";
      LC_ALL = "en_US.UTF-8";
      ZSH_THEME_TERM_TAB_TITLE_IDLE = "🐚 %~";
      ZSH_THEME_TERM_TITLE_IDLE = "🐚 %~";
    };
    oh-my-zsh = {
      enable = true;
      plugins = [
        "golang"
        "kubectl"
        "node"
        "python"
        "ruby"
        "rust"
        "terraform"
      ];
    };
    initContent = ''
      ${builtins.readFile ./init.zsh}
      ${builtins.readFile ./terminal.zsh}
    '';
  };
}
