# The Windows Daemon

niri's windows, held ready for the launcher over
`$XDG_RUNTIME_DIR/daemons/windows.sock`, in the shell's daemon interface
(version 1, as the programs and machine daemons speak it). The shell
imports it, so a machine with the shell runs it.

It hears niri's event stream on `NIRI_SOCKET` for as long as it runs and
works every record out again at each event, which takes microseconds
here. A watch is told only the records that changed and the keys gone,
so a focus moving tells the two windows it moved between, a window
resizing or the view scrolling tells nothing, and the shell remakes only
the rows a change names. Before it, each event niri sent woke the
shell's main thread to walk every window, look each one's program up and
make its row again, shut or open (Joel's shell, 2026-11-10: 657 window
rows made in four minutes with the launcher shut).

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
|                     | `inside`: its windows' programs, `new`: niri's empty last one | Move to Workspace's rows |
| `desktop`           | the window the keys were in, with its title and program, the workspace and output in front, the overview, whether niri's settings failed to load | Capture |
|                     | `spaces`: each output's workspaces, in order    | Move to Workspace's order     |

A window's program is the desktop entry its app id names, as Quickshell's
`heuristicLookup` finds it: by its id, its id in any case, its class,
then its class in any case, the first by id where two match alike. A
browser's window of its own that names none counts for the listed entry
whose own page it opened deeper in (BrowserWindows.js said it so).

## Left Out, and Why

- **Its tile's size and place in the view.** They change at every resize
  and scroll, and no row draws them.
- **Words.** "workspace 2", "3rd column" and the rest are the shell's,
  from what each record says.
- **Acting on a window.** The shell asks niri itself, on niri's own
  socket, so an act never waits on this daemon.
