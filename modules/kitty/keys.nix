# kitty's shortcuts, as data: each chord, what it does in plain words, and
# kitty's action for it. home.nix renders them into keys.conf and adds them
# to the keyboard map, which refuses one the desktop above already takes
# (modules/keys).
#
# They're WezTerm's defaults and nothing else: Ctrl+Shift for the terminal,
# so the program inside keeps every plain Ctrl chord, and Ctrl with Tab,
# Page Up and Down, equals, minus and 0 where every terminal and browser
# puts them.
{ lib }:
let
  numbered = make: lib.genList (index: make (toString (index + 1))) 9;
in
[
  {
    chord = "ctrl+shift+c";
    does = "Copy";
    action = "copy_to_clipboard";
  }
  {
    chord = "ctrl+shift+v";
    does = "Paste";
    action = "paste_from_clipboard";
  }
  {
    chord = "shift+insert";
    does = "Paste the selection";
    action = "paste_from_selection";
  }

  {
    chord = "ctrl+shift+t";
    does = "Open a tab here";
    action = "new_tab_with_cwd";
  }
  {
    chord = "ctrl+shift+w";
    does = "Close the tab";
    action = "close_tab";
  }
  {
    chord = "ctrl+shift+n";
    does = "Open a window here";
    action = "new_os_window_with_cwd";
  }
  {
    chord = "ctrl+tab";
    does = "Go to the next tab";
    action = "next_tab";
  }
  {
    chord = "ctrl+shift+tab";
    does = "Go to the previous tab";
    action = "previous_tab";
  }
  {
    chord = "ctrl+page_down";
    does = "Go to the next tab";
    action = "next_tab";
  }
  {
    chord = "ctrl+page_up";
    does = "Go to the previous tab";
    action = "previous_tab";
  }
  {
    chord = "ctrl+shift+page_down";
    does = "Move the tab right";
    action = "move_tab_forward";
  }
  {
    chord = "ctrl+shift+page_up";
    does = "Move the tab left";
    action = "move_tab_backward";
  }
]
++ numbered (number: {
  chord = "ctrl+shift+${number}";
  does = "Go to tab ${number}";
  action = "goto_tab ${number}";
})
++ [
  {
    chord = "shift+page_up";
    does = "Scroll up a page";
    action = "scroll_page_up";
  }
  {
    chord = "shift+page_down";
    does = "Scroll down a page";
    action = "scroll_page_down";
  }
  {
    chord = "ctrl+shift+home";
    does = "Scroll to the top";
    action = "scroll_home";
  }
  {
    chord = "ctrl+shift+end";
    does = "Scroll to the bottom";
    action = "scroll_end";
  }
  {
    chord = "ctrl+shift+f";
    does = "Search the scrollback";
    action = "search_scrollback";
  }
  {
    chord = "ctrl+shift+x";
    does = "Read the scrollback in a pager";
    action = "show_scrollback";
  }
  {
    chord = "ctrl+shift+z";
    does = "Read the last command's output in a pager";
    action = "show_last_command_output";
  }
  # WezTerm's quick select: pick a word, path or hash from the screen, or a
  # link.
  {
    chord = "ctrl+shift+space";
    does = "Pick a word from the screen";
    action = "kitten hints --type word --program @";
  }
  {
    chord = "ctrl+shift+e";
    does = "Open a link on the screen";
    action = "kitten hints --type url";
  }
  {
    chord = "ctrl+shift+p";
    does = "Find a terminal action";
    action = "command_palette";
  }
  {
    chord = "ctrl+shift+u";
    does = "Type a character by its name";
    action = "kitten unicode_input";
  }

  {
    chord = "ctrl+equal";
    does = "Make the text larger";
    action = "change_font_size all +1.0";
  }
  {
    chord = "ctrl+minus";
    does = "Make the text smaller";
    action = "change_font_size all -1.0";
  }
  {
    chord = "ctrl+0";
    does = "Put the text back to its size";
    action = "change_font_size all 0";
  }
  {
    chord = "ctrl+shift+r";
    does = "Read the configuration again";
    action = "load_config_file";
  }
]
