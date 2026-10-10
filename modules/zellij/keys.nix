# zellij's chords in its resting mode, locked, where every other key goes to
# the program inside, and in normal mode too: each chord, what it does in
# plain words, and zellij's action for it. home.nix renders them into
# config.kdl and adds them to the keyboard map, which refuses one the
# desktop or the terminal above already takes (modules/keys). The modes
# entered from here keep their own single letters in config.kdl, since
# nothing above sees those.
#
# Ctrl Alt is zellij's, as Ctrl Shift is the terminal's, and Ctrl g leaves
# the resting mode for the others.
[
  {
    chord = "Ctrl g";
    does = "Unlock zellij's other modes";
    action = ''SwitchToMode "Normal";'';
    onlyLocked = true;
  }
  {
    chord = "Ctrl Alt n";
    does = "Open a pane";
    action = "NewPane;";
  }
  {
    chord = "Ctrl Alt f";
    does = "Show the floating panes, or hide them";
    action = "ToggleFloatingPanes;";
  }
  {
    chord = "Ctrl Alt z";
    does = "Choose a project to work on";
    action = ''
      Run "zdev" "-n" {
          name "Choose a project"
          floating true
          close_on_exit true
          x "20%"
          y "30%"
          width "60%"
          height "38%"
      }
    '';
  }
  {
    chord = "Ctrl Alt i";
    does = "Move the tab left";
    action = ''MoveTab "Left";'';
  }
  {
    chord = "Ctrl Alt o";
    does = "Move the tab right";
    action = ''MoveTab "Right";'';
  }
  {
    chord = "Ctrl Alt Left";
    does = "Focus the pane or tab to the left";
    action = ''MoveFocusOrTab "Left";'';
  }
  {
    chord = "Ctrl Alt Right";
    does = "Focus the pane or tab to the right";
    action = ''MoveFocusOrTab "Right";'';
  }
  {
    chord = "Ctrl Alt Down";
    does = "Focus the pane below";
    action = ''MoveFocus "Down";'';
  }
  {
    chord = "Ctrl Alt Up";
    does = "Focus the pane above";
    action = ''MoveFocus "Up";'';
  }
  {
    chord = "Ctrl Alt =";
    does = "Grow the pane";
    action = ''Resize "Increase";'';
  }
  {
    chord = "Ctrl Alt +";
    does = "Grow the pane";
    action = ''Resize "Increase";'';
  }
  {
    chord = "Ctrl Alt -";
    does = "Shrink the pane";
    action = ''Resize "Decrease";'';
  }
  {
    chord = "Ctrl Alt [";
    does = "Use the previous layout";
    action = "PreviousSwapLayout;";
  }
  {
    chord = "Ctrl Alt ]";
    does = "Use the next layout";
    action = "NextSwapLayout;";
  }
]
