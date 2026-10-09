# zsh as the system sets it up on either NixOS or a Mac, for the shell the
# zsh home module configures in each account.
{
  programs.zsh = {
    enable = true;
    # oh-my-zsh runs compinit itself; a second global one walks fpath twice.
    enableGlobalCompInit = false;
    # powerlevel10k owns the prompt.
    promptInit = "";
  };
}
