{ pkgs, ... }:
{
  programs.claude-code = {
    enable = true;
    settings = {
      feedbackSurveyState = {
        lastShownTime = 1754753318519;
      };
      statusLine = {
        type = "command";
        command = "claude-statusline";
        padding = 0;
      };
    };
  };

  home.packages = with pkgs; [
    (writeScriptBin "claude-statusline" (builtins.readFile ./claude-statusline.rb))
  ];

  programs.git.ignores = [
    "CLAUDE.local.md"
  ];
}