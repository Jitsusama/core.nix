# What every account's home-manager does, whatever the machine.
{
  programs.home-manager.enable = true;
  # Tools that install themselves for one user put their commands here.
  home.sessionPath = [ "$HOME/.local/bin" ];
}
