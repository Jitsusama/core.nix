# delta, git's pager for diffs, so a diff reads as a file does in bat: its
# code in bat's theme, and its lines and words tinted from the success and
# alert colours as Claude Code's diffs are.
{ config, lib, ... }:
let
  inherit (config.jitsusama.theme) colours;
  mix = import ../theme/mix.nix { inherit lib; };
  tint = weight: colour: mix weight colour colours.background;
in
{
  imports = [
    ../bat/home.nix
    ../git/home.nix
    ../theme/home.nix
  ];

  programs.delta = {
    enable = true;
    enableGitIntegration = true;
    # A machine that leaves bat out leaves delta its own syntax colours.
    options =
      lib.optionalAttrs (config.programs.bat.config ? theme) {
        syntax-theme = config.programs.bat.config.theme;
      }
      // {
        # n and N step from one file to the next in the pager.
        navigate = true;
        file-style = "${colours.strong} bold";
        file-decoration-style = "${colours.rule} ul";
        # The file is named once, above its hunks.
        hunk-header-style = "line-number syntax";
        hunk-header-decoration-style = "none";
        hunk-header-line-number-style = colours.muted;
        line-numbers-zero-style = colours.muted;
        line-numbers-minus-style = colours.alert;
        line-numbers-plus-style = colours.success;
        minus-style = "syntax ${tint 0.18 colours.alert}";
        minus-emph-style = "syntax ${tint 0.35 colours.alert}";
        plus-style = "syntax ${tint 0.25 colours.success}";
        plus-emph-style = "syntax ${tint 0.45 colours.success}";
        blame-palette = "${colours.background} ${colours.sunken}";
      };
  };
}
