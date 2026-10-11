# zellij's two status bars, drawn by zjstatus from jitsusama.theme: the tabs
# along the top, and along the bottom the mode with the keys it offers, so a
# mode always shows and explains itself (docs/design.md, no hidden modes).
# Each mode's hints are data here, the one copy both layouts draw.
# zjstatus's guide to its format: https://github.com/dj95/zjstatus/wiki
{ lib, theme }:
let
  inherit (theme) roles glyphs;

  plugin = ''location="https://github.com/dj95/zjstatus/releases/latest/download/zjstatus.wasm"'';

  # Every mode but locked, where keys go to the program, is a mode Joel is in,
  # so its name takes the accent; locked is the resting state and stays quiet.
  modes = {
    normal = {
      label = "NORMAL";
      keys = [
        [
          "C-g"
          "lock"
        ]
        [
          "p"
          "pane"
        ]
        [
          "t"
          "tab"
        ]
        [
          "r"
          "resize"
        ]
        [
          "s"
          "scroll"
        ]
        [
          "o"
          "session"
        ]
        [
          "m"
          "move"
        ]
        [
          "C-q"
          "quit"
        ]
      ];
    };
    locked = {
      label = "LOCKED";
      resting = true;
      keys = [
        [
          "C-g"
          "unlock"
        ]
        [
          "C-Alt-n"
          "new pane"
        ]
        [
          "C-Alt-←→↑↓"
          "move focus"
        ]
        [
          "C-Alt-+-"
          "resize"
        ]
      ];
    };
    resize = {
      label = "RESIZE";
      keys = [
        [
          "hjkl"
          "resize"
        ]
        [
          "HJKL"
          "resize more"
        ]
        [
          "="
          "increase"
        ]
        [
          "-"
          "decrease"
        ]
        [
          "r"
          "back"
        ]
      ];
    };
    pane = {
      label = "PANE";
      keys = [
        [
          "hjkl"
          "focus"
        ]
        [
          "n"
          "new"
        ]
        [
          "d"
          "down"
        ]
        [
          "r"
          "right"
        ]
        [
          "x"
          "close"
        ]
        [
          "f"
          "fullscreen"
        ]
        [
          "z"
          "frames"
        ]
        [
          "w"
          "float"
        ]
        [
          "e"
          "embed"
        ]
        [
          "c"
          "rename"
        ]
        [
          "i"
          "pin"
        ]
        [
          "p"
          "back"
        ]
      ];
    };
    tab = {
      label = "TAB";
      keys = [
        [
          "hl"
          "switch"
        ]
        [
          "n"
          "new"
        ]
        [
          "x"
          "close"
        ]
        [
          "r"
          "rename"
        ]
        [
          "s"
          "sync"
        ]
        [
          "b"
          "break"
        ]
        [
          "]"
          "break right"
        ]
        [
          "["
          "break left"
        ]
        [
          "1-9"
          "go to"
        ]
        [
          "Tab"
          "toggle"
        ]
        [
          "t"
          "back"
        ]
      ];
    };
    scroll = {
      label = "SCROLL";
      keys = [
        [
          "jk"
          "scroll"
        ]
        [
          "C-f"
          "page down"
        ]
        [
          "C-b"
          "page up"
        ]
        [
          "d"
          "half down"
        ]
        [
          "u"
          "half up"
        ]
        [
          "e"
          "edit"
        ]
        [
          "f"
          "search"
        ]
        [
          "C-c"
          "bottom"
        ]
        [
          "s"
          "back"
        ]
      ];
    };
    enter_search = {
      label = "SEARCH";
      say = "type to search";
      keys = [
        [
          "Enter"
          "confirm"
        ]
        [
          "C-c"
          "cancel"
        ]
      ];
    };
    search = {
      label = "SEARCH";
      keys = [
        [
          "n"
          "next"
        ]
        [
          "p"
          "previous"
        ]
        [
          "c"
          "case"
        ]
        [
          "w"
          "word"
        ]
        [
          "o"
          "wrap"
        ]
        [
          "C-c"
          "exit"
        ]
      ];
    };
    rename_tab = {
      label = "RENAME";
      say = "type a new name";
      keys = [
        [
          "Enter"
          "confirm"
        ]
      ];
    };
    rename_pane = {
      label = "RENAME";
      say = "type a new name";
      keys = [
        [
          "Enter"
          "confirm"
        ]
      ];
    };
    session = {
      label = "SESSION";
      keys = [
        [
          "d"
          "detach"
        ]
        [
          "w"
          "session manager"
        ]
        [
          "c"
          "configuration"
        ]
        [
          "p"
          "plugin manager"
        ]
        [
          "a"
          "about"
        ]
        [
          "o"
          "back"
        ]
      ];
    };
    move = {
      label = "MOVE";
      keys = [
        [
          "hjkl"
          "move pane"
        ]
        [
          "n"
          "move"
        ]
        [
          "p"
          "move backwards"
        ]
        [
          "m"
          "back"
        ]
      ];
    };
    prompt = {
      label = "PROMPT";
      say = "answer the prompt";
      keys = [ ];
    };
    tmux = {
      label = "TMUX";
      keys = [
        [
          "C-b"
          "send prefix"
        ]
      ];
    };
  };

  # A key in the text's colour and weight, what it does in the text's own,
  # and the theme's separator, quiet, between one hint and the next.
  # zjstatus starts each #[...] afresh, so a style lasts until the next one.
  hint =
    pair: "#[fg=${roles.text},bold]${lib.elemAt pair 0}#[fg=${roles.muted}] ${lib.elemAt pair 1}";
  separator = "#[fg=${roles.rule}] ${glyphs.separator.glyph} ";
  # Every label is padded to the longest, so the hints start in one column
  # whatever the mode.
  padded = label: label + lib.concatStrings (lib.genList (_: " ") (9 - lib.stringLength label));
  mode =
    m:
    let
      color = if m.resting or false then roles.muted else roles.accent;
    in
    "#[fg=${color},bold]${padded m.label}"
    + lib.concatStringsSep separator (
      lib.optional (m ? say) "#[fg=${roles.muted}]${m.say}" ++ map hint m.keys
    );
in
{
  tabs = ''
    pane size=1 borderless=true {
        plugin ${plugin} {
            format_left   " {tabs}"
            format_center ""
            format_right  ""
            format_space  ""
            border_enabled  "false"
            hide_frame_for_single_pane "false"

            // The tab in view is the strong one; the rest are muted.
            tab_normal   "#[fg=${roles.muted}] {name} "
            tab_active   "#[fg=${roles.strong},bold] {name} "
            tab_separator ""
            tab_sync_indicator       " ${glyphs.synced.glyph}"
            tab_fullscreen_indicator " ${glyphs.fullscreen.glyph}"
            tab_floating_indicator   " ${glyphs.floating.glyph}"
        }
    }
  '';

  modes = ''
    pane size=1 borderless=true {
        plugin ${plugin} {
            format_left   " {mode}"
            format_center ""
            format_right  ""
            format_space  ""
            border_enabled  "false"
            hide_frame_for_single_pane "false"

    ${lib.concatLines (lib.mapAttrsToList (name: m: "        mode_${name} \"${mode m}\"") modes)}
        }
    }
  '';
}
