{
  programs.home-manager.enable = true;
  # Tools that install themselves for one user put their commands here.
  home.sessionPath = [ "$HOME/.local/bin" ];
}
