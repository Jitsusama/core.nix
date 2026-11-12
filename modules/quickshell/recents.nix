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
      each time the launcher opens, and with `told` too, read again only
      once the program says its list changed (kitty's tells.py); `list`,
      the command that prints it, newest first; `dated`, true where each line it prints ends in a tab
      and when it was last used in seconds since 1970, or nothing where
      that isn't known, so a program's Recent places it among its other
      sources by when, and lists one with no time after those with one
      (launcher/Recent.js); and `open` and `start`, how Return opens a path ({path})
      through the process behind the window used last ({pid}) while one is
      open, and while none is, a site opening by the program's own command.
      A site source may say what's open now too: `opened`, the command
      printing a tab a line (its id, which `go` and `close` are given, or
      nothing where neither can act on it, its window, whether it's in front
      there, when it was last looked at, its address and its title, a tab
      between each), read as the program is stepped into; or, as Chrome's
      does, `session`, its profile's folder, which the programs daemon reads
      those tabs from itself, with no command, as it's stepped into and as
      Chrome writes the session while it's watched, asking the extension's
      host on its socket first where `host` is true; and where it
      can, `go` and `close`, the commands going to and closing one ({id}).
      Without `go`, Go To brings forward the window the tab is in; without
      `close`, there's no Close It. A file source may say what's open now
      too: `opened`, the command printing the process that has a file open
      and the file's whole path, a tab between, a file a line, read as the
      program is stepped into; Go To and Close It act on that process's
      window. Where one process has many windows, as a terminal has for
      what runs in each, a line names niri's window instead, as `@` and its id, and
      after another tab what `go` is given ({id}) to bring the file forward
      inside that window before Go To brings the window forward; such a
      file has no Close It. One with only `app`, `kind` and `opened` keeps
      the list programs share for its program's other paths. One with a
      `name` is a page's own list rather than its program's Recent, as
      Chrome's downloads are: the programs daemon holds the lines its
      `list` prints as they are, under `listed:` and its `app` and `name`,
      read as that key is watched, so the page starts no process for it.
      One with a `name` may give `folders` in place of `list`: the daemon
      reads what's in them itself, a file a line newest first, its time in
      seconds, its size and its path, a tab between, as find's
      `%T@\t%s\t%p` prints them, and again on every change in one, as
      what was taken lately is (Taken.qml).
    '';
  };
}
