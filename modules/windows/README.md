# The Windows Daemon

niri's windows, held ready for the launcher over
`$XDG_RUNTIME_DIR/daemons/windows.sock`, in the shell's daemon interface
(version 1, as the programs and machine daemons speak it). The shell
imports it, so a machine with the shell runs it.

It hears niri's event stream on `NIRI_SOCKET` for as long as it runs and
works every record out again at each event, which takes microseconds
here. A watch is told only the records that changed and the keys gone,
so a focus moving tells the two windows it moved between, a window
resizing or the view scrolling tells only a watch that names `layout`,
and the shell remakes only
the rows a change names. Before it, each event niri sent woke the
shell's main thread to walk every window, look each one's program up and
make its row again, shut or open (Joel's shell, 2026-11-10: 657 window
rows made in four minutes with the launcher shut).

A watch may name `fields`, and is then told each record as its key and
those fields alone, and only when one of them changed. The shell holds
one on when each window was used while the launcher is shut, so Super+Tab's
first frame lists them in that order, and a retitle wakes nothing.

## Every Figure, and Where It Shows

| Kind                | Figures                                         | Where                         |
| ------------------- | ----------------------------------------------- | ----------------------------- |
| `window:<niri id>`  | its app id as the shell knows it, and as opened | the row's key, its glyph      |
|                     | whether it holds a session                      | Agents' join                  |
|                     | title, and `named`: without its program's name  | the row's name                |
|                     | `untitled`: whether that's its program's alone  | whether it's its program's row |
|                     | its entry's id, and its program's name          | the row's side, Shift+Return  |
|                     | `owner`: the program it counts for              | each program's windows        |
|                     | `alike`: how many windows are named the same    | whether the row says its column |
|                     | workspace, its number, whether it's elsewhere   | the row's side and context    |
|                     | column, and how far down it                     | the row's side, when alike    |
|                     | floating, urgent, its process                   | its verbs, its program's rows |
|                     | `focused`: whether the keys were in it          | This Window, Super+Tab        |
|                     | `order`: when it was last looked at, in ms      | where its row is placed       |
| `workspace:<id>`    | number, name, output, active, focused, urgent, its active window | the verbs that move a window |
|                     | `inside`: its programs, each once with its count, `new`: niri's empty last one | Move to Workspace's rows |
| `desktop`           | the window the keys were in, with its title and program, the workspace and output in front, the overview, whether niri's settings failed to load | Capture |
|                     | `spaces`: each output's workspaces, in order    | Move to Workspace's order     |
| `focus`             | the window niri says has the keys, none while a layer such as the launcher has them | the shell's niri (Niri.qml) |
| `cast:<stream id>`  | each cast as niri says it: its kind, whether it's active, its session, process and target | the recorder, the bar's sharing mark, a call |
| `layout:<niri id>`  | each window's tile as niri lays it out: its place in the view, its size and the window's | where a notice opens below, the recorder's size |
| `screenshot`        | the last screenshot's path, and `taken`: how many so far | the recorder |

A watch or list that names no kind gets every kind but `layout`, which
changes at every resize and scroll, and `focus`, which changes at every
opening of the launcher, so only a watch that draws tiles, or follows
niri's focus, is woken by them.

A window's program is the desktop entry its app id names, as Quickshell's
`heuristicLookup` finds it: by its id, its id in any case, its class,
then its class in any case, the first by id where two match alike. A
browser's window of its own that names none counts for the listed entry
whose own page it opened deeper in (BrowserWindows.js said it so).

## Left Out, and Why

- **A row's tile.** A window's record says its column and how far down,
  not its tile's size or place, which change at every resize and scroll;
  those are its `layout` record's, for the few that draw them.
- **Words.** "workspace 2", "3rd column" and the rest are the shell's,
  from what each record says.
- **Acting on a window.** The shell asks niri itself, on niri's own
  socket, so an act never waits on this daemon.
