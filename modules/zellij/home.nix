# zellij, Joel's terminal multiplexer: config.kdl says how it behaves, and
# its theme and its layouts' status bars are written here from
# jitsusama.theme, so it looks like everything else.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  theme = config.jitsusama.theme;
  inherit (theme) roles terminal;
  status = import ./status.nix { inherit lib theme; };
in
{
  imports = [ ../theme/home.nix ];

  home.packages = [ (pkgs.writeScriptBin "zdev" (builtins.readFile ./zdev.zsh)) ];
  programs.zellij.enable = true;

  xdg.configFile = {
    "zellij/config.kdl".source = ./config.kdl;

    # zellij's own colours, for its frames and its built-in screens.
    "zellij/themes/jitsusama.kdl".text = ''
      themes {
          jitsusama {
              fg "${roles.text}"
              bg "${roles.selection}"
              black "${terminal.black}"
              red "${roles.alert}"
              green "${roles.accent}"
              yellow "${roles.warning}"
              blue "${terminal.blue}"
              magenta "${terminal.magenta}"
              cyan "${terminal.cyan}"
              white "${roles.strong}"
              orange "${terminal.bright_yellow}"
          }
      }
    '';

    # The tabs along the top, a plain pane, and the mode along the bottom.
    "zellij/layouts/default.kdl".text = ''
      layout {
          default_tab_template {
      ${status.tabs}
              children
      ${status.modes}
          }
          tab {
              pane
          }
      }
    '';

    # zdev's layout: a coding agent on the left, and Neovim stacked over a
    # shell on the right. Usage: zdev, or zellij action new-tab --layout zdev.
    "zellij/layouts/zdev.kdl".text = ''
      layout {
      ${status.tabs}
          pane split_direction="vertical" {
              pane size="50%" focus=true {
                  command "claude"
              }
              pane size="50%" stacked=true {
                  pane {
                      command "nvim"
                  }
                  pane
              }
          }
      ${status.modes}
      }
    '';
  };
}
