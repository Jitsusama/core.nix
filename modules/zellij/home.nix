# zellij, Joel's terminal multiplexer: config.kdl says how it behaves, and
# its theme and its layouts' status bars are written here from
# jitsusama.theme, so it looks like everything else. Its resting chords are
# jitsusama.zellij.keys, rendered into config.kdl and added to the keyboard
# map (modules/keys).
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

  keys = config.jitsusama.zellij.keys;
  # Each line one level further in, as KDL is written.
  indent =
    text:
    lib.concatStringsSep "\n" (
      map (line: if line == "" then "" else "    " + line) (lib.splitString "\n" text)
    );
  binds = lib.concatMapStrings (
    key: "bind ${builtins.toJSON key.chord} {\n${indent (lib.trim key.action)}\n}\n"
  );
  block = name: chosen: "${name} {\n${indent (binds chosen)}}\n";
  resting = indent (
    block "locked" (lib.filter (key: key.onlyLocked) keys)
    + block ''shared_among "normal" "locked"'' (lib.filter (key: !key.onlyLocked) keys)
  );
  marker = "    // @resting@\n";
  written = builtins.readFile ./config.kdl;
in
{
  imports = [
    ../theme/home.nix
    ../keys/home.nix
  ];

  options.jitsusama.zellij.keys = lib.mkOption {
    type = lib.types.listOf (
      lib.types.submodule {
        options = {
          chord = lib.mkOption {
            type = lib.types.str;
            description = "The chord, as zellij spells it: Ctrl Alt n.";
          };
          does = lib.mkOption {
            type = lib.types.str;
            description = "What it does in plain words, as the launcher finds it.";
          };
          action = lib.mkOption {
            type = lib.types.str;
            description = "zellij's actions, as config.kdl would spell them: NewPane;";
          };
          onlyLocked = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "Whether it works only in the locked mode, not in normal mode too.";
          };
        };
      }
    );
    default = [ ];
    description = ''
      zellij's chords in its resting mode, keys.nix's to begin with. A
      machine's own are added to them; lib.mkForce replaces them all.
    '';
  };

  config.jitsusama.zellij.keys = import ./keys.nix;

  config.jitsusama.keys.multiplexer = map (key: { inherit (key) chord does; }) keys;

  config.home.packages = [ (pkgs.writeScriptBin "zdev" (builtins.readFile ./zdev.zsh)) ];
  config.programs.zellij.enable = true;

  config.xdg.configFile = {
    "zellij/config.kdl".text =
      assert lib.assertMsg (lib.hasInfix marker written)
        "zellij's config.kdl lost the line its resting chords go on";
      lib.replaceStrings [ marker ] [ resting ] written;

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
