# The commands a viewer's : gives the launcher's command line, for it to
# open over the viewer (Launcher.qml's viewerCommandLine). Each viewer's own
# module adds its own, as it adds its keys, so they live beside the program
# that does them.
{ lib, ... }:
{
  options.jitsusama.quickshell.launcher.commandLines = lib.mkOption {
    type = lib.types.attrsOf (lib.types.attrsOf lib.types.anything);
    default = { };
    description = ''
      Each viewer's command line, by the name its : gives `launcher-say
      command-line`: `commands`, each line of its menu, `command` the
      command's whole name as the key language names it, `title` what it
      does and `key` the key that does it too; and `run`, the program that
      carries a command to the viewer, given where the viewer is, as its :
      said it, the command's whole name and what was typed after it, so
      :{n} and a time come as goto and what was typed.
    '';
  };
}
