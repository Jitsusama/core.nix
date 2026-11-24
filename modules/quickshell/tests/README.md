# The Shell's Cases

What the shell's checks in [`checks/`][1] run: QML cases that hold one of
its JavaScript files to what it says in Qt's own engine, the scripts and
pytest cases that drive the whole shell on a nested niri, and the
stand-ins in `stand-ins/` that answer for the daemons it reads.

Many entries because each case is named for the behaviour it proves, and
each check reads its case by that name, so a failing check leads straight
to the file. Every shell case that runs off a VM is here; what it shares
with other checks, such as the nested niri it runs on, is in the top-level
`tests/lib/`, and the stand-ins it borrows from other checks are in the
repository's `tests/stand-ins/` and `tests/desktop/stand-ins/`.

- `answers/`: each place's cases for the launcher's answers, which
  `answers.qml` runs in turn, as its README says.
- `compiled-once/`: a shell of three files, started three times by
  `compiled-once.sh` to show Qt compiles each once and keeps it.
- `screen-picture/`: a shell showing the wallpaper's picture as its
  screen goes and comes back, as its README says.

[1]: ../checks/
