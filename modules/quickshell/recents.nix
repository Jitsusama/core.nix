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
      what each path is, a file unless it says folder, or a site, an address
      and a title a line; `watch`, the file read again each time it
      changes, or `asked`, read only as the program is stepped into, once
      each time the launcher opens; `list`, the command that prints it,
      newest first; and `open` and `start`, how Return opens a path ({path})
      through the process behind the window used last ({pid}) while one is
      open, and while none is, a site opening by the program's own command.
      A site source may say what's open now too: `opened`, the command
      printing a tab a line (its id, its window, whether it's in front
      there, when it was last looked at, its address and its title, a tab
      between each), read as the program is stepped into; and where it
      can, `go` and `close`, the commands going to and closing one ({id}).
      Without `go`, Go To brings forward the window the tab is in; without
      `close`, there's no Close It. A file source may say what's open now
      too: `opened`, the command printing the process that has a file open
      and the file's whole path, a tab between, a file a line, read as the
      program is stepped into; Go To and Close It act on that process's
      window. Where one process has many windows, as kitty has for each
      Neovim, a line names niri's window instead, as `@` and its id, and
      after another tab what `go` is given ({id}) to bring the file forward
      inside that window before Go To brings the window forward; such a
      file has no Close It. One with only `app`, `kind` and `opened` keeps
      the list programs share for its program's other paths.
    '';
  };
}
