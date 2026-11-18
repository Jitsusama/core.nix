# The Windows Daemon

niri's windows, held ready for the launcher over
`$XDG_RUNTIME_DIR/daemons/windows.sock`, in the shell's daemon interface
(version 1, as the programs and machine daemons speak it), answered by
the serving side every daemon shares (`../daemon-serve`), so its socket,
watches and health are theirs. The shell imports it, so a machine with
the shell runs it.

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

A boot's first ask of its socket can start it before niri has named
`NIRI_SOCKET` to the user manager. It then waits, asking the user
manager's environment every two seconds, and starts listening once niri
is named, rather than ending with an error each boot (src/socket.rs).

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
|                     | and, a session's, without pi's leading mark     | the row's name, Super+S's line |
|                     | `untitled`: whether that's its program's alone  | whether it's its program's row |
|                     | its entry's id, and its program's name          | the row's side, Shift+Return  |
|                     | `owner`: the program it counts for              | each program's windows        |
|                     | `alike`: how many windows are named the same    | whether the row says its column |
|                     | workspace, its number, whether it's elsewhere   | the row's side and context    |
|                     | column, and how far down it                     | the row's side, when alike    |
|                     | floating, urgent, its process                   | its verbs, its program's rows |
|                     | `file`: a Reader's, the whole path it shows     | the void's line about it      |
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
| `row:displays:<id>` | each screen as Screens shows it, worded, placed and ready, in the form every daemon's rows take (../ready-rows), then Turn Every Screen Off while one is lit | the launcher's Screens |

A watch or list that names no kind gets every kind but `layout`, which
changes at every resize and scroll, `focus`, which changes at every
opening of the launcher, and `row`, which is the launcher's alone, so
only a watch that draws tiles, follows niri's focus or shows Screens is
woken by them. A watch of rows may name its `place`, `displays`.

The rows are made only where the shell hands the daemon the launcher's
words (`--words`, its words.json). A word the file lacks, or every word
where it won't read, is said by its id, and warned of once in the
journal, so Screens keeps its rows and acts for want of a word, as the
sound daemon's do. Each screen's row carries its drawer: its connector
over its name, then what niri says of it past its row, then what its
EDID says as desk-facts writes it (`$XDG_RUNTIME_DIR/desk/displays.json`,
heard with inotify as it's written again), niri's size where the EDID
gives none, and its serial last, which identifies it. Figures are said
in the account's number settings (`--numbers`, `{ group, point }`).
Each of a lit screen's settings, and Turn Every Screen Off, carries a
drawer too: what it does, what a setting offers or which screens it
turns off, and how the : line types it, as `:setlocal scale=1.5`.

The screen holding the Brightness, the one its backlight lights or the
monitor following the panel, leads its drawer with its light, read from
the machine daemon (`--machine`, its machine.sock) only while a watch's
`keys` name a screen's row, as the shell's watch ahead does while one is
selected or stepped into. The machine daemon reads the light every
second it's watched, so nothing watches it while no screen is looked at.
Its first second unread says nothing, then "not read yet"; the night
light's sunset or sunrise travels as a time for the shell to word, and
the drawer is said again as either passes.

niri says nothing as a screen comes,
goes or turns off, so the daemon asks niri for its outputs as the stream
begins and at each workspace change, which each of those makes, and
tells a watch only the rows that moved. A change the shell makes to a
screen, its scale or its mode, makes no event at all, nor does a key
waking the screens, so the shell then, and as the launcher opens,
asks the daemon to `look` (`{"id": 1, "ask": "look"}`): it asks niri
again, and tells each watch what moved before answering
`{"id": 1, "looked": true}`, or `false` where it makes no rows.

A window's program is the desktop entry its app id names, as Quickshell's
`heuristicLookup` finds it: by its id, its id in any case, its class,
then its class in any case, the first by id where two match alike. A
browser's window of its own that names none counts for the listed entry
whose own page it opened deeper in (BrowserWindows.js said it so).

## How Long a Program Takes

The daemon tells the journal how long each program took to open and to
end, for health to read (src/timed.rs):

```text
opened foot 212 ms after its process started
closed foot; its process ended 14 ms later
closed zoom; its process was still running 30 s later
```

An opening is a process's first window within a minute of its start, read
from /proc, so a new window of a program already running, Chrome's or a
session in kitty's one instance, says nothing. A closing is a process's
last window gone, and its process is then looked at every 20 ms for the
first 400 ms, then after a tenth of the time waited so far, on a thread
that ends with the wait, for up to 30 s: a program kept in the
background outlives every wait, and looking every 20 ms throughout cost
it 1,500 wakes where the growing pause costs about 65. niri's event stream says
nothing of the key that asked a window to close, so a quit is timed from
the window gone, not from the ask.

## A Window an Agent Opened

Agents run where polkit's rules say they do (modules/slices/agents-ask-first.rules):
agents.slice, jobs.service, jobs-work.slice, jobs-timed.slice, shared.slice
and pi-jobs scopes. A window whose process sits there was opened by an agent
over Joel's work. What Joel starts at a terminal runs in jobs-awaited.slice
and is his. The daemon says each one to the journal as it opens, as a
warning (src/unasked.rs):

```text
an agent opened a window over Joel's work: "org.kde.heaptrack" "Heaptrack", from
pi-host-work-background.service in "/home/joel/src/tree"
```

The sessions' own windows, in kitty's one instance in agents.slice, are
never said, nor is anything the launcher opens, which starts in
app.slice. A window there as the event stream begins was said when it
opened. The line names the unit and its folder, so the agent behind it can
be found.

One that asks for a password is closed as it opens: a keyring's prompter,
gpg's pinentry, ssh's askpass, KWallet or a polkit agent, known by the app
id each names itself with. On 2026-11-17 an agent's test started a keyring
of its own, whose prompt opened on Joel's screen, and he typed his real
password into it. The daemon asks niri to close the window, says so in the
journal, and puts up a notice naming the scope and folder it came from,
through libnotify's notify-send. niri asks the window to close as its own
close key does, so the notice says it was asked to; a prompter could
ignore it. An agent rerunning its test makes a fresh scope each time, so
Joel is told once a minute for a folder at most. Where niri refuses, the
notice says the window couldn't be closed and to type no password there,
and the prompt is asked again when niri next says it; one that never
changes again isn't. A prompt is weighed
whenever niri says it, so one open as the daemon starts again, as after a
switch, or one naming itself late, is closed too.
Joel's own prompts come from his services and stay. The installed daemon
reads processes from /proc alone; only the check's build reads a stand-in.

Any other window an agent opens that takes the focus as it opens is put
behind Joel's work (src/behind.rs): niri moves it, unfocused, to the empty
workspace it keeps last on that screen. First the last of Joel's own
windows that had the focus has it again, never another agent's, and only
while the agent's window still holds it, so a window he focused himself
meanwhile keeps it. It's like what niri does for a sandbox's windows
(modules/niri/sandboxed-windows-stay-behind.patch), for agents outside one,
though a window put behind can still ask to be activated later. The window
shows for as long as niri takes to answer, a frame or two, so a key Joel
presses then can reach it. One that opens unfocused, or takes the focus
later, stays where it opened, as do the plain kitty windows Joel opens from
a pi window, which the sessions' kitty runs. An X11 window is missed, as an
X11 prompter is below.

It stops an agent's accident, not an agent set on getting past it: the
cgroup is a label an agent could change. Three gaps are known. gpg-agent
starts on demand from whoever first runs gpg, so an agent that started it
would make Joel's pinentry look like the agent's, and it would be closed,
the notice saying where from. An X11 prompter is missed, since niri gives
the pid of xwayland-satellite, in niri's own unit. And a prompter the
session bus starts runs in Joel's units, so it stays.

## Left Out, and Why

- **A row's tile.** A window's record says its column and how far down,
  not its tile's size or place, which change at every resize and scroll;
  those are its `layout` record's, for the few that draw them.
- **Words for a window.** "workspace 2", "3rd column" and the rest are
  the shell's, from what each record says, until windows' rows are
  served ready as Screens' are.
- **Acting on a window or a screen.** The shell asks niri itself, on
  niri's own socket, so an act never waits on this daemon. A screen's row
  holds its connector and how it stands (`held`), never niri's whole
  record of it, so a change niri says that moves nothing the row says
  makes no row again, and the shell asks niri nothing of a screen itself.
