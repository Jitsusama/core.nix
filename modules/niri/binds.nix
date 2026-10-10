# niri's shortcuts, as data: each chord, what it does in plain words, and
# niri's action for it. home.nix renders them into binds.kdl, names each in
# niri's own list of shortcuts with its words, and adds them to the keyboard
# map, which checks them against every layer below (modules/keys).
#
# The keys follow Joel's own habits, from his i3, zellij and Neovim
# configurations, and one grammar so a chord can be worked out rather than
# remembered:
#
# - Super is the desktop, and the arrows move focus. Focus flows over an
#   edge the way niri lays things out: up and down run on into the next
#   workspace, left and right into the next screen.
# - Shift carries the column along with focus, Alt carries just the window
#   through the grid of columns and the windows stacked in them, and Ctrl
#   reaches across to another screen.
# - A window opens in a column of its own, as niri opens it, unless it's
#   asked to open below the focused one.
# - A letter keeps its meaning with a modifier: F fills, R sizes, M widens,
#   C centres.
# - Nothing ends the session on a bare chord.
{ lib }:
let
  numbered = make: lib.genList (index: make (toString (index + 1))) 9;
  locked = {
    allow-when-locked = "true";
  };
  # A wheel sends many steps for one flick, so it waits between them.
  wheel = {
    cooldown-ms = "150";
  };
in
# Programs, as in i3. The launcher lives in Quickshell, which also holds
# locking and ending the session, so no chord ends it by accident.
[
  {
    chord = "Super+Return";
    does = "Open a terminal";
    action = ''spawn "kitty";'';
  }
  {
    chord = "Super+Shift+Return";
    does = "Open a terminal below this window";
    action = ''spawn "qs" "ipc" "call" "grid" "below" "kitty";'';
  }
  {
    chord = "Super+Alt+Return";
    does = "Open the next window below this one";
    action = ''spawn "qs" "ipc" "call" "grid" "below" "";'';
  }
  {
    chord = "Super+Space";
    does = "Find a program or an action";
    action = ''spawn "qs" "ipc" "call" "launcher" "toggle";'';
  }
  {
    chord = "Super+Shift+Slash";
    does = "Show these shortcuts";
    action = "show-hotkey-overlay;";
  }

  # Closing is destructive, so it takes Shift, as in i3.
  {
    chord = "Super+Shift+Q";
    does = "Close the window";
    action = "close-window;";
    flags.repeat = "false";
  }

  # Focus.
  {
    chord = "Super+Left";
    does = "Focus the column to the left, or the screen past it";
    action = "focus-column-or-monitor-left;";
  }
  {
    chord = "Super+Right";
    does = "Focus the column to the right, or the screen past it";
    action = "focus-column-or-monitor-right;";
  }
  {
    chord = "Super+Up";
    does = "Focus the window above, or the workspace past it";
    action = "focus-window-or-workspace-up;";
  }
  {
    chord = "Super+Down";
    does = "Focus the window below, or the workspace past it";
    action = "focus-window-or-workspace-down;";
  }
  {
    chord = "Super+Home";
    does = "Focus the first column";
    action = "focus-column-first;";
  }
  {
    chord = "Super+End";
    does = "Focus the last column";
    action = "focus-column-last;";
  }
  {
    chord = "Super+Grave";
    does = "Go back to the window before";
    action = "focus-window-previous;";
  }
  {
    chord = "Super+Shift+Space";
    does = "Focus the floating windows, or the tiled ones";
    action = "switch-focus-between-floating-and-tiling;";
  }

  # Shift carries the column, or the window up and down within it.
  {
    chord = "Super+Shift+Left";
    does = "Move the column left, or onto the screen past it";
    action = "move-column-left-or-to-monitor-left;";
  }
  {
    chord = "Super+Shift+Right";
    does = "Move the column right, or onto the screen past it";
    action = "move-column-right-or-to-monitor-right;";
  }
  {
    chord = "Super+Shift+Up";
    does = "Move the window up, or onto the workspace past it";
    action = "move-window-up-or-to-workspace-up;";
  }
  {
    chord = "Super+Shift+Down";
    does = "Move the window down, or onto the workspace past it";
    action = "move-window-down-or-to-workspace-down;";
  }
  {
    chord = "Super+Shift+Home";
    does = "Move the column to the start";
    action = "move-column-to-first;";
  }
  {
    chord = "Super+Shift+End";
    does = "Move the column to the end";
    action = "move-column-to-last;";
  }

  # Alt carries just the window through the grid: sideways into the next
  # column, below what's there, or out into a column of its own when it
  # shares one; up and down within its column. The brackets swap it with
  # the window beside it instead.
  {
    chord = "Super+Alt+Left";
    does = "Move the window into the column on the left, or out of its own";
    action = "consume-or-expel-window-left;";
  }
  {
    chord = "Super+Alt+Right";
    does = "Move the window into the column on the right, or out of its own";
    action = "consume-or-expel-window-right;";
  }
  {
    chord = "Super+Alt+Up";
    does = "Move the window up its column";
    action = "move-window-up;";
  }
  {
    chord = "Super+Alt+Down";
    does = "Move the window down its column";
    action = "move-window-down;";
  }
  {
    chord = "Super+BracketLeft";
    does = "Swap the window with the one to the left";
    action = "swap-window-left;";
  }
  {
    chord = "Super+BracketRight";
    does = "Swap the window with the one to the right";
    action = "swap-window-right;";
  }
  {
    chord = "Super+Comma";
    does = "Take the window to the right into this column";
    action = "consume-window-into-column;";
  }
  {
    chord = "Super+Period";
    does = "Put the column's bottom window into a column of its own";
    action = "expel-window-from-column;";
  }

  # Ctrl reaches across to the other screens.
  {
    chord = "Super+Ctrl+Left";
    does = "Focus the screen to the left";
    action = "focus-monitor-left;";
  }
  {
    chord = "Super+Ctrl+Right";
    does = "Focus the screen to the right";
    action = "focus-monitor-right;";
  }
  {
    chord = "Super+Ctrl+Up";
    does = "Focus the screen above";
    action = "focus-monitor-up;";
  }
  {
    chord = "Super+Ctrl+Down";
    does = "Focus the screen below";
    action = "focus-monitor-down;";
  }
  {
    chord = "Super+Ctrl+Shift+Left";
    does = "Move the column to the screen to the left";
    action = "move-column-to-monitor-left;";
  }
  {
    chord = "Super+Ctrl+Shift+Right";
    does = "Move the column to the screen to the right";
    action = "move-column-to-monitor-right;";
  }
  {
    chord = "Super+Ctrl+Shift+Up";
    does = "Move the column to the screen above";
    action = "move-column-to-monitor-up;";
  }
  {
    chord = "Super+Ctrl+Shift+Down";
    does = "Move the column to the screen below";
    action = "move-column-to-monitor-down;";
  }
  {
    chord = "Super+Ctrl+Alt+Left";
    does = "Move the window to the screen to the left";
    action = "move-window-to-monitor-left;";
  }
  {
    chord = "Super+Ctrl+Alt+Right";
    does = "Move the window to the screen to the right";
    action = "move-window-to-monitor-right;";
  }
  {
    chord = "Super+Ctrl+Alt+Up";
    does = "Move the window to the screen above";
    action = "move-window-to-monitor-up;";
  }
  {
    chord = "Super+Ctrl+Alt+Down";
    does = "Move the window to the screen below";
    action = "move-window-to-monitor-down;";
  }
]

# Workspaces by number, as in i3; Page Up and Down walk through them.
++ numbered (number: {
  chord = "Super+${number}";
  does = "Go to workspace ${number}";
  action = "focus-workspace ${number};";
})
++ numbered (number: {
  chord = "Super+Shift+${number}";
  does = "Move the column to workspace ${number}";
  action = "move-column-to-workspace ${number};";
})
++ numbered (number: {
  chord = "Super+Alt+${number}";
  does = "Move the window to workspace ${number}";
  action = "move-window-to-workspace ${number};";
})
++ [
  {
    chord = "Super+Page_Up";
    does = "Go to the workspace above";
    action = "focus-workspace-up;";
  }
  {
    chord = "Super+Page_Down";
    does = "Go to the workspace below";
    action = "focus-workspace-down;";
  }
  {
    chord = "Super+Shift+Page_Up";
    does = "Move the column to the workspace above";
    action = "move-column-to-workspace-up;";
  }
  {
    chord = "Super+Shift+Page_Down";
    does = "Move the column to the workspace below";
    action = "move-column-to-workspace-down;";
  }
  {
    chord = "Super+Alt+Page_Up";
    does = "Move the window to the workspace above";
    action = "move-window-to-workspace-up;";
  }
  {
    chord = "Super+Alt+Page_Down";
    does = "Move the window to the workspace below";
    action = "move-window-to-workspace-down;";
  }
  {
    chord = "Super+Tab";
    does = "Show every workspace";
    action = "toggle-overview;";
    flags.repeat = "false";
  }

  # The wheel walks the same way the arrows do: down and up through the
  # workspaces, sideways through the columns, and Shift turns the wheel
  # sideways for a mouse that can't.
  {
    chord = "Super+WheelScrollDown";
    does = "Go to the workspace below";
    action = "focus-workspace-down;";
    flags = wheel;
  }
  {
    chord = "Super+WheelScrollUp";
    does = "Go to the workspace above";
    action = "focus-workspace-up;";
    flags = wheel;
  }
  {
    chord = "Super+WheelScrollRight";
    does = "Focus the column to the right";
    action = "focus-column-right;";
  }
  {
    chord = "Super+WheelScrollLeft";
    does = "Focus the column to the left";
    action = "focus-column-left;";
  }
  {
    chord = "Super+Shift+WheelScrollDown";
    does = "Focus the column to the right";
    action = "focus-column-right;";
  }
  {
    chord = "Super+Shift+WheelScrollUp";
    does = "Focus the column to the left";
    action = "focus-column-left;";
  }

  # Sizes: r cycles the presets, as resizing was r in i3 and zellij; equals
  # and minus grow and shrink, as in zellij. Shift turns each to the height,
  # and Shift with 0 hands the height back to niri.
  {
    chord = "Super+R";
    does = "Cycle the column's width";
    action = "switch-preset-column-width;";
  }
  {
    chord = "Super+Shift+R";
    does = "Cycle the window's height";
    action = "switch-preset-window-height;";
  }
  {
    chord = "Super+Equal";
    does = "Widen the column";
    action = ''set-column-width "+10%";'';
  }
  {
    chord = "Super+Minus";
    does = "Narrow the column";
    action = ''set-column-width "-10%";'';
  }
  {
    chord = "Super+Shift+Equal";
    does = "Make the window taller";
    action = ''set-window-height "+10%";'';
  }
  {
    chord = "Super+Shift+Minus";
    does = "Make the window shorter";
    action = ''set-window-height "-10%";'';
  }
  {
    chord = "Super+Shift+0";
    does = "Share the column's height evenly again";
    action = "reset-window-height;";
  }
  {
    chord = "Super+M";
    does = "Make the column as wide as the screen, or put it back";
    action = "maximize-column;";
  }
  {
    chord = "Super+Shift+M";
    does = "Widen the column into the space beside it";
    action = "expand-column-to-available-width;";
  }

  # Layout, with i3's letters where niri has the same idea.
  {
    chord = "Super+F";
    does = "Fill the screen with the window, or put it back";
    action = "fullscreen-window;";
  }
  {
    chord = "Super+Shift+F";
    does = "Float the window, or tile it again";
    action = "toggle-window-floating;";
  }
  {
    chord = "Super+Alt+F";
    does = "Tell the window it fills the screen, but keep it in its tile";
    action = "toggle-windowed-fullscreen;";
  }
  {
    chord = "Super+E";
    does = "Stretch the window to the screen's edges, or put it back";
    action = "maximize-window-to-edges;";
  }
  {
    chord = "Super+W";
    does = "Show the column's windows as tabs, or stacked";
    action = "toggle-column-tabbed-display;";
  }
  {
    chord = "Super+C";
    does = "Centre the column";
    action = "center-column;";
  }
  {
    chord = "Super+Shift+C";
    does = "Centre every column in view";
    action = "center-visible-columns;";
  }

  # Hand every key to the focused program, as zellij's locked mode does, for
  # a virtual machine or a remote desktop. The same chord gives them back.
  {
    chord = "Super+Escape";
    does = "Give every key to the window, or take them back";
    action = "toggle-keyboard-shortcuts-inhibit;";
    flags.allow-inhibiting = "false";
  }

  # Hardware keys, which also work at the lock screen.
  {
    chord = "XF86AudioRaiseVolume";
    does = "Turn the volume up";
    action = ''spawn-sh "wpctl set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ 5%+";'';
    flags = locked;
  }
  {
    chord = "XF86AudioLowerVolume";
    does = "Turn the volume down";
    action = ''spawn-sh "wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-";'';
    flags = locked;
  }
  {
    chord = "XF86AudioMute";
    does = "Mute the speakers, or unmute them";
    action = ''spawn-sh "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle";'';
    flags = locked;
  }
  {
    chord = "XF86AudioMicMute";
    does = "Mute the microphone, or unmute it";
    action = ''spawn-sh "wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle";'';
    flags = locked;
  }
  {
    chord = "XF86AudioPlay";
    does = "Play or pause";
    action = ''spawn "playerctl" "play-pause";'';
    flags = locked;
  }
  {
    chord = "XF86AudioNext";
    does = "Play the next track";
    action = ''spawn "playerctl" "next";'';
    flags = locked;
  }
  {
    chord = "XF86AudioPrev";
    does = "Play the previous track";
    action = ''spawn "playerctl" "previous";'';
    flags = locked;
  }
  {
    chord = "XF86MonBrightnessUp";
    does = "Brighten the screen";
    action = ''spawn "brightnessctl" "--class=backlight" "set" "+5%";'';
    flags = locked;
  }
  {
    chord = "XF86MonBrightnessDown";
    does = "Dim the screen";
    action = ''spawn "brightnessctl" "--class=backlight" "set" "5%-";'';
    flags = locked;
  }

  # Screenshots: pick a region, the whole screen, or one window.
  {
    chord = "Print";
    does = "Take a screenshot of a region";
    action = "screenshot;";
  }
  {
    chord = "Shift+Print";
    does = "Take a screenshot of the screen";
    action = "screenshot-screen;";
  }
  {
    chord = "Ctrl+Print";
    does = "Take a screenshot of the window";
    action = "screenshot-window;";
  }
]
