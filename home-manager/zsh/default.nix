{
  pkgs,
  ...
}:
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
    plugins = [
      {
        name = "powerlevel10k";
        src = pkgs.zsh-powerlevel10k;
        file = "share/zsh-powerlevel10k/powerlevel10k.zsh-theme";
      }
    ];
    initContent = ''
      ${builtins.readFile ./init.zsh}
      ${builtins.readFile ./terminal.zsh}
      ${builtins.readFile ./powerlevel10k.zsh}
    '';
  };
}

