# Where programs keep what they had open lately, for each program's Recent
# in the launcher (launcher/Recents.qml). Each program's own module adds
# where its list is, as it hands its keys to the launcher through
# ../keys/home.nix, so a list lives beside the program that keeps it.
{ lib, ... }:
{
  options.jitsusama.quickshell.launcher.recents = lib.mkOption {
    type = lib.types.listOf (lib.types.attrsOf lib.types.anything);
    default = [ ];
    description = ''
      Where programs keep what they had open lately, each added by the
      module of the program it's for: `app`, the desktop entry it's for, or
      none for a list programs share, a program and a path a line; `kind`,
      what each path is, a file unless it says folder; `watch`,
      the file read again each time it changes; `list`, the command that
      prints it, newest first; and `open` and `start`, how Return opens a
      path ({path}) through the process behind the window used last ({pid})
      while one is open, and while none is.
    '';
  };
}
