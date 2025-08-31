{
  ...
}:
{
  programs.zsh = {
    enable = true;
    shellAliases = {
      cat = "bat";
    };
    sessionVariables = {
      LANG = "en_US.UTF-8";
      LC_ALL = "en_US.UTF-8";
    };
    oh-my-zsh = {
      enable = true;
      custom = "$HOME/.oh-my-zsh";
      plugins = [
        "brew"
        "git"
        "golang"
        "kubectl"
        "node"
        "python"
        "ruby"
        "rust"
        "terraform"
      ];
    };
    initContent = builtins.readFile ./init.zsh;
  };
}