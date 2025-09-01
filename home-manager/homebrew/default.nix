{
  ...
}:
{
  programs.zsh = {
    oh-my-zsh.plugins = [ "brew" ];
    initContent = builtins.readFile ./init.zsh;
  };
}

