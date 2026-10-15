# Homebrew's programs in each Mac account's shell: its shellenv puts them on
# PATH and its completions on fpath, so it runs before completion starts.
{ lib, ... }:
{
  programs.zsh.initContent = lib.mkOrder 550 (builtins.readFile ./init.zsh);
}
