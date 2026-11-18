# The Shell's Cases

What the shell's checks in [`checks/`][1] run: QML cases that hold one of
its JavaScript files to what it says in Qt's own engine, the scripts and
pytest cases that drive the whole shell on a nested niri, and the
stand-ins in `stand-ins/` that answer for the daemons it reads.

Many entries because each case is named for the behaviour it proves, and
each check reads its case by that name, so a failing check leads straight
to the file. A case that a VM test or another module reads too, or that
reads a shared piece in the top-level `tests/` as it runs by a path from
its own folder, rather than being handed it, waits there until that piece
moves. A path given only to a linter doesn't count.

- `compiled-once/`: a shell of three files, started three times by
  `compiled-once.sh` to show Qt compiles each once and keeps it.
- `screen-picture/`: a shell showing the wallpaper's picture as its
  screen goes and comes back, as its README says.

[1]: ../checks/
