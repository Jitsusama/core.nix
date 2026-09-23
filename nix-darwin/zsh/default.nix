{ ... }:
{
  programs.zsh = {
    # oh-my-zsh runs compinit itself; a second global one walks fpath twice.
    enableGlobalCompInit = false;

    # powerlevel10k owns the prompt.
    promptInit = "";
  };
}
