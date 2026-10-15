# zsh as the system sets it up on either NixOS or a Mac, for the shell the
# zsh home module configures in each account.
{
  programs.zsh = {
    enable = true;
    # The account's zsh runs compinit itself, from a kept dump
    # (completion.zsh); a second global one walks fpath again.
    enableGlobalCompInit = false;
    # The prompt module owns the prompt.
    promptInit = "";
  };
}
