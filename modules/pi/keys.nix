# pi's keys, one home for each: pi's keybindings file is written from these,
# so pi takes exactly them, and the key map lists them (jitsusama.keys.pi)
# beside Neovim's, where the launcher finds them by what they do.
#
# pi is a text box, so its keys are the ones a text box has: plain Ctrl, as
# in every shell, with Escape to back out. The terminal's keys are
# Ctrl+Shift, and pi takes only the three kitty leaves it, for its own full
# screen. Each entry is a chord, what it does in plain words, and the pi
# actions it's bound to; a chord bound to two does each where it applies.
# The file gives each action named here only the chords listed for it, so
# an action keeps every key it should have here, and one not named keeps
# pi's own.
# pi's own names are listed at https://pi.dev/schemas/keybindings.schema.json.
[
  {
    chord = "escape";
    does = "Stop pi, or back out";
    actions = [ "app.interrupt" ];
  }
  {
    chord = "enter";
    does = "Send the message";
    actions = [ "tui.input.submit" ];
  }
  {
    chord = "shift+enter";
    does = "Start a new line";
    actions = [ "tui.input.newLine" ];
  }
  {
    chord = "ctrl+j";
    does = "Start a new line, where Shift+Return reads as Return";
    actions = [ "tui.input.newLine" ];
  }
  {
    chord = "alt+enter";
    does = "Send the message once pi's done";
    actions = [ "app.message.followUp" ];
  }
  {
    chord = "alt+up";
    does = "Take back the messages waiting to go";
    actions = [ "app.message.dequeue" ];
  }
  {
    chord = "ctrl+c";
    does = "Copy the selection, or clear what's typed, and quit when pressed twice";
    actions = [
      "tui.input.copy"
      "app.clear"
    ];
  }
  {
    chord = "ctrl+d";
    does = "Delete the character ahead, or quit when nothing's typed";
    actions = [
      "tui.editor.deleteCharForward"
      "app.exit"
    ];
  }
  {
    chord = "delete";
    does = "Delete the character ahead";
    actions = [ "tui.editor.deleteCharForward" ];
  }
  {
    chord = "ctrl+z";
    does = "Put pi in the background";
    actions = [ "app.suspend" ];
  }
  {
    chord = "ctrl+g";
    does = "Write the message in Neovim";
    actions = [ "app.editor.external" ];
  }
  {
    chord = "ctrl+v";
    does = "Paste an image, or text";
    actions = [ "app.clipboard.pasteImage" ];
  }
  {
    chord = "ctrl+-";
    does = "Undo typing";
    actions = [ "tui.editor.undo" ];
  }
  {
    chord = "ctrl+y";
    does = "Paste what was deleted last";
    actions = [ "tui.editor.yank" ];
  }
  {
    chord = "ctrl+l";
    does = "Choose a model";
    actions = [ "app.model.select" ];
  }
  {
    chord = "ctrl+p";
    does = "Go to the next model";
    actions = [ "app.model.cycleForward" ];
  }
  {
    chord = "ctrl+shift+p";
    does = "Go to the model before";
    actions = [ "app.model.cycleBackward" ];
  }
  {
    chord = "shift+tab";
    does = "Change how hard pi thinks";
    actions = [ "app.thinking.cycle" ];
  }
  {
    chord = "ctrl+t";
    does = "Show pi's thinking, or hide it";
    actions = [ "app.thinking.toggle" ];
  }
  {
    chord = "ctrl+o";
    does = "Show what tools said, or fold it away";
    actions = [ "app.tools.expand" ];
  }
  {
    chord = "ctrl+x";
    does = "Copy pi's last answer";
    actions = [ "app.message.copy" ];
  }
  {
    chord = "pageUp";
    does = "Scroll up a page";
    actions = [
      "tui.editor.pageUp"
      "tui.altScreen.pageUp"
    ];
  }
  {
    chord = "pageDown";
    does = "Scroll down a page";
    actions = [
      "tui.editor.pageDown"
      "tui.altScreen.pageDown"
    ];
  }
  {
    chord = "ctrl+home";
    does = "Go to the start of the conversation";
    actions = [ "tui.altScreen.top" ];
    when = "in full screen";
  }
  {
    chord = "ctrl+end";
    does = "Go to the end of the conversation, and follow it";
    actions = [ "tui.altScreen.bottom" ];
    when = "in full screen";
  }
  {
    chord = "ctrl+shift+up";
    does = "Go to your message before";
    actions = [ "tui.altScreen.previousPrompt" ];
    when = "in full screen";
  }
  {
    chord = "ctrl+shift+down";
    does = "Go to your message after";
    actions = [ "tui.altScreen.nextPrompt" ];
    when = "in full screen";
  }
  {
    chord = "ctrl+shift+f";
    does = "Find in the conversation";
    actions = [ "tui.altScreen.search" ];
    when = "in full screen";
  }
]
