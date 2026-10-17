# niri's shortcuts, as data: each chord, what it does in plain words, and
# niri's action for it. home.nix renders them into binds.kdl, names each in
# niri's own list of shortcuts with its words, and adds them to the keyboard
# map, which checks them against every layer below (modules/keys).
#
# The keys speak the language every layer does (docs/design.md, The
# Keyboard), so a chord can be worked out rather than remembered:
#
# - Super is the desktop, and the arrows move focus. Focus flows over an
#   edge the way niri lays things out: up and down run on into the next
#   workspace, left and right into the next screen.
# - Shift carries the thing at that level: the column with the arrows, the
#   workspace with Page Up and Down. Alt carries just the window, and Ctrl
#   reaches across to another screen.
# - The words mean what they mean in every layer: Return opens, W closes,
#   Space finds, ? shows what's here, Tab goes back to the one used last.
# - A letter keeps its meaning with a modifier: F fills, R sizes, M widens,
#   C centres, T shows tabs.
# - What's done often ends on a key both of Joel's keyboards have without a
#   layer or Fn: a letter, an arrow, Tab, Return, Space or Escape. Page Up,
#   Home, Print and the symbols are for what's done rarely.
# - A window opens in a column of its own, as niri opens it, unless it's
#   asked to open below the focused one.
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
# ending the session, so no chord ends it by accident. Locking loses
# nothing, so it has a chord.
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
  # The margin's drawer: what's on, Keep Awake or do not disturb or the mic
  # muted, and what can be done about it. O is a home key, under the right
  # hand while the left thumb holds Super.
  {
    chord = "Super+O";
    does = "Show what's on";
    action = ''spawn "qs" "ipc" "call" "margin" "toggle";'';
  }
  # The launcher's largest collections, each opened straight. Tab is the one
  # used last: every other window, that one first, so Super+Tab and Return
  # go back to it and a few letters find the rest, with nothing held.
  {
    chord = "Super+Tab";
    does = "Find a window";
    action = ''spawn "qs" "ipc" "call" "launcher" "collection" "windows";'';
  }
  {
    chord = "Super+V";
    does = "Find something copied earlier";
    action = ''spawn "qs" "ipc" "call" "launcher" "collection" "clipboard";'';
  }
  {
    chord = "Super+E";
    does = "Find an emoji";
    action = ''spawn "qs" "ipc" "call" "launcher" "collection" "emoji";'';
  }
  {
    chord = "Super+N";
    does = "Find a notice that came earlier";
    action = ''spawn "qs" "ipc" "call" "launcher" "collection" "notices";'';
  }
  # What AI use cost (modules/spend-watch). U for usage.
  {
    chord = "Super+U";
    does = "Find what AI use cost";
    action = ''spawn "qs" "ipc" "call" "launcher" "collection" "spend";'';
  }
  # The cards on screen put away, as a click on each would: each stays
  # unread where Super+N finds it.
  {
    chord = "Super+Shift+N";
    does = "Put the notices away";
    action = ''spawn "qs" "ipc" "call" "notices" "putAway";'';
  }
  {
    chord = "Super+L";
    does = "Lock the screen";
    action = ''spawn "qs" "ipc" "call" "lock" "lock";'';
  }
  {
    chord = "Super+Shift+Slash";
    does = "Show these shortcuts";
    # The launcher's collection of every layer's shortcuts, searchable, in
    # place of niri's own overlay of only its own.
    action = ''spawn "qs" "ipc" "call" "launcher" "collection" "shortcuts";'';
  }
  # The keyboard's layout, for a glance while typing, where a keyboard's
  # module describes one. K for keyboard.
  {
    chord = "Super+K";
    does = "Show the keyboard's layout";
    action = ''spawn "qs" "ipc" "call" "keyboard" "toggle";'';
  }

  # W closes in every layer, as Cmd+W did on the Mac. A program asks before
  # it loses work.
  {
    chord = "Super+W";
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
    chord = "Super+Shift+Space";
    does = "Focus the floating windows, or the tiled ones";
    action = "switch-focus-between-floating-and-tiling;";
  }

  # Shift carries the column: sideways, and up and down onto the workspace
  # past it.
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
    does = "Move the column to the workspace above";
    action = "move-column-to-workspace-up;";
  }
  {
    chord = "Super+Shift+Down";
    does = "Move the column to the workspace below";
    action = "move-column-to-workspace-down;";
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
  # shares one; up and down its column, and on onto the workspace past it.
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
    does = "Move the window up, or onto the workspace past it";
    action = "move-window-up-or-to-workspace-up;";
  }
  {
    chord = "Super+Alt+Down";
    does = "Move the window down, or onto the workspace past it";
    action = "move-window-down-or-to-workspace-down;";
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

# Workspaces by number, as in i3; Page Up and Down walk through them, and
# with Shift carry the workspace itself, as Shift carries the thing at its
# level. They need Fn on the laptop, so what's done often is on the arrows
# too.
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
    does = "Move the workspace up";
    action = "move-workspace-up;";
  }
  {
    chord = "Super+Shift+Page_Down";
    does = "Move the workspace down";
    action = "move-workspace-down;";
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
  # Ctrl reaches another screen here too, and Shift still carries the
  # workspace: screens sit side by side, so up and down mean the one before
  # and the one after.
  {
    chord = "Super+Ctrl+Shift+Page_Up";
    does = "Move the workspace to the screen before";
    action = "move-workspace-to-monitor-previous;";
  }
  {
    chord = "Super+Ctrl+Shift+Page_Down";
    does = "Move the workspace to the screen after";
    action = "move-workspace-to-monitor-next;";
  }
  # A for all.
  {
    chord = "Super+A";
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

  # Sizes: r cycles the presets, as resizing was r in i3; equals and minus
  # grow and shrink, as they do the text in a terminal or a browser. Shift
  # turns each to the height, and Shift with 0 hands the height back to
  # niri.
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
    does = "Give the window back its share of the column's height";
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
  {
    chord = "Super+Alt+M";
    does = "Stretch the window to the screen's edges, or put it back";
    action = "maximize-window-to-edges;";
  }

  # Layout, with i3's letters where niri has the same idea, and G, for
  # grab, lifting a window out to float so F only ever fills.
  {
    chord = "Super+F";
    does = "Fill the screen with the window, or put it back";
    action = "fullscreen-window;";
  }
  {
    chord = "Super+G";
    does = "Float the window, or tile it again";
    action = "toggle-window-floating;";
  }
  {
    chord = "Super+Alt+F";
    does = "Tell the window it fills the screen, but keep it in its tile";
    action = "toggle-windowed-fullscreen;";
  }
  {
    chord = "Super+T";
    does = "Show the column's windows as tabs, or stacked";
    action = "toggle-column-tabbed-display;";
  }
  {
    chord = "Super+C";
    # A tiled window is centred with its column, a floating one alone.
    does = "Centre the window";
    action = "center-window;";
  }
  {
    chord = "Super+Shift+C";
    does = "Centre every column in view";
    action = "center-visible-columns;";
  }

  # Hand every key to the focused program, for a virtual machine or a
  # remote desktop. The same chord gives them back.
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
    # The shell's music (Music.qml), so the keys act on the player it
    # shows, and show what they did.
    chord = "XF86AudioPlay";
    does = "Play or pause the music";
    action = ''spawn "qs" "ipc" "call" "music" "toggle";'';
    flags = locked;
  }
  {
    chord = "XF86AudioNext";
    does = "Play the next track";
    action = ''spawn "qs" "ipc" "call" "music" "next";'';
    flags = locked;
  }
  {
    chord = "XF86AudioPrev";
    does = "Play the previous track";
    action = ''spawn "qs" "ipc" "call" "music" "previous";'';
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
  {
    # The keyboard's lock key, which macOS reads as locking the screen.
    chord = "XF86ScreenSaver";
    does = "Lock the screen";
    action = ''spawn "qs" "ipc" "call" "lock" "lock";'';
  }
  {
    # The keyboard's Do Not Disturb key, which holds notices until it's
    # pressed again, as macOS's Focus key does (Quiet.qml).
    chord = "XF86DoNotDisturb";
    does = "Hold notices, or let them show";
    action = ''spawn "qs" "ipc" "call" "quiet" "toggle";'';
  }

  # Print means capturing something: it opens the launcher's Capture
  # collection, where the whole screen comes first, so Print then Return
  # takes it, and a window, a region, a recording or the camera is found by
  # typing (Quickshell's launcher/Capture.qml). A region is picked by the
  # keys (Picker.qml), since niri's own picker takes only the pointer. The
  # chords after it are shortcuts to Capture's rows, shown beside them.
  # The ErgoDox has Print only on a layer, so Super+P opens Capture too,
  # first so it's the chord the launcher shows.
  {
    chord = "Super+P";
    does = "Capture the screen, a window, a region or the camera";
    action = ''spawn "qs" "ipc" "call" "launcher" "collection" "capture";'';
  }
  {
    chord = "Print";
    does = "Capture the screen, a window, a region or the camera";
    action = ''spawn "qs" "ipc" "call" "launcher" "collection" "capture";'';
  }
  {
    chord = "Shift+Print";
    does = "Take a screenshot of the screen";
    action = "screenshot-screen;";
  }
  {
    chord = "Alt+Print";
    does = "Take a screenshot of the window";
    action = "screenshot-window;";
  }

  # A recording: Ctrl makes Print's picture a video, and Alt makes it the
  # window's, as it does the picture's. The chord that starts one stops it
  # (Quickshell's Recorder.qml). A region is the launcher's to record.
  {
    chord = "Ctrl+Print";
    does = "Record the screen, or stop recording";
    action = ''spawn "qs" "ipc" "call" "recording" "screen";'';
  }
  {
    chord = "Ctrl+Alt+Print";
    does = "Record the window, or stop recording";
    action = ''spawn "qs" "ipc" "call" "recording" "window";'';
  }

  # Sharing, for a call that shares niri's switchable target, which a
  # program asks for once and which shows what these chords point it at:
  # Super moves Print from a picture to the share, with Ctrl for the window
  # and Shift for the screen as above. The share starts blank, so nothing is
  # shown before it's pointed at something.
  {
    chord = "Super+Ctrl+Print";
    does = "Switch the share to this window";
    action = "set-dynamic-cast-window;";
  }
  {
    chord = "Super+Shift+Print";
    does = "Switch the share to this screen";
    action = "set-dynamic-cast-monitor;";
  }
  {
    chord = "Super+Print";
    does = "Blank the share";
    action = "clear-dynamic-cast-target;";
  }
]
