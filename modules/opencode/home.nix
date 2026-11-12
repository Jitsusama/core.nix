# opencode, a coding agent for the terminal, kept at the version Nix gives.
{
  programs.opencode = {
    enable = true;
    settings.autoupdate = false;
    # OpenCode 2 reads its terminal's settings from cli.json, not tui.json.
    cli.theme.name = "gruvbox";
  };
}
