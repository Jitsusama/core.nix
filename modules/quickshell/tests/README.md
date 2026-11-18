# The Shell's Cases

What the shell's checks in [`checks/`][1] run: QML cases that hold one of
its JavaScript files to what it says in Qt's own engine, the scripts and
pytest cases that drive the whole shell on a nested niri, and the
stand-ins in `stand-ins/` that answer for the daemons it reads.

Many entries because each case is named for the behaviour it proves, and
each check reads its case by that name, so a failing check leads straight
to the file. A case that a VM test or another module reads too, or that
reads a shared piece in the top-level `tests/` by its own folder, waits
there until that piece moves.

[1]: ../checks/
