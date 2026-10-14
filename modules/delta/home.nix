# delta, git's pager for diffs, so a diff reads as a file does in bat: its
# code in bat's theme, and its lines and words tinted from the success and
# alert colours as Claude Code's diffs are.
{ config, lib, ... }:
let
  inherit (config.jitsusama.theme) roles;
  mix = import ../theme/mix.nix { inherit lib; };
  tint = weight: colour: mix weight colour roles.background;
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
        file-style = "${roles.strong} bold";
        file-decoration-style = "${roles.rule} ul";
        # The file is named once, above its hunks.
        hunk-header-style = "line-number syntax";
        hunk-header-decoration-style = "none";
        hunk-header-line-number-style = roles.muted;
        line-numbers-zero-style = roles.muted;
        line-numbers-minus-style = roles.alert;
        line-numbers-plus-style = roles.success;
        minus-style = "syntax ${tint 0.18 roles.alert}";
        minus-emph-style = "syntax ${tint 0.35 roles.alert}";
        plus-style = "syntax ${tint 0.25 roles.success}";
        plus-emph-style = "syntax ${tint 0.45 roles.success}";
        blame-palette = "${roles.background} ${roles.sunken}";
      };
  };
}
