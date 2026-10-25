# opencode, a coding agent for the terminal, kept at the version Nix gives.
{
  programs.opencode = {
    enable = true;
    settings.autoupdate = false;
    tui.theme = "gruvbox";
  };
}
