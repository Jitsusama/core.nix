# The Programs Daemon

What's installed and what each program had open, held ready for the
launcher over `$XDG_RUNTIME_DIR/daemons/programs.sock`, in the shell's
daemon interface (version 1, as the people and machine daemons speak
it). The shell imports it, so a machine with the shell runs it.

It reads nothing while nobody watches, and nothing on a timer: an entry
as its key is first asked and again when inotify says its folder
changed; a program's list as a watch of it begins, if its file changed
since, and on each change while it's watched; what a program has open
as it's asked. Every command a source names runs without a shell, on a
thread of its own, and is stopped past ten seconds.

## Every Figure, and Where It Shows

| Kind                 | Figures                                        | Where                         |
| -------------------- | ---------------------------------------------- | ----------------------------- |
| `program:<entry id>` | name, generic name, comment                    | Programs' rows, the drawer's body |
|                      | categories, keywords                           | the drawer's Categories, Keywords |
|                      | exec as its arguments, terminal, starts in     | the drawer's Command, Starts In |
|                      | window class, entry path                       | the drawer's identifiers      |
|                      | actions, each its name and exec                | the program's rows            |
|                      | what it opens: each type, its globs, its description | the drawer's Opens      |
|                      | the links it handles, by scheme                | the drawer's Handles          |
|                      | its package: name, version, store path         | the drawer's Version, Package |
| `recent:<entry id>`  | each path or site, when it was last used       | the program's Recent          |
| `recent:shared`      | each program's path, when, from the shared list | each program's Recent        |
| `opened:<entry id>`  | each file's process or window, path, Go To's id | the program's open rows      |
|                      | each tab's id, window, front, last look, address, title | the program's open rows |

An entry's id is its file's path under `applications` with `/` made
`-` and no `.desktop`, as Quickshell gives it; the nearest data folder
holding one wins, and one saying `Hidden` is none.

## Left Out, and Why

- **Icon.** The row draws it already.
- **TryExec, StartupNotify, DBusActivatable, NoDisplay, OnlyShowIn,
  NotShowIn and every `X-` key.** They say how and where a program is
  started, not what it is.
- **Localized keys.** Joel reads English.
- **A package's size and when it was installed.** Only the Nix daemon
  knows them, asked through a command for each package.
