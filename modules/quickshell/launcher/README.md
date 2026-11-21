# The Launcher's Providers

One provider per kind of thing the launcher finds, each starting from
[`Provider.qml`][provider], with the `.js` rules it works out apart from
drawing beside it. Beside them are the pieces several providers share, such
as `ReadyRows.qml`, `Recents.qml` and `Previewer.qml`, the word lists and
tables some read (`words/`, `*.nix`), and the programs that make those
tables (`*.py`). Each file's opening comment says what it is for, and [the
shell's guide][guide] says how the launcher fits together.

Many entries because every file here is named by where it sits: by its
neighbours' relative imports (`import "Order.js" as Order`), through `qs`,
by the shell's views above it, by the install lists and by the checks that
copy files one at a time. Moving one rewrites every place that names it, so
the folder is split into kinds at a quiet moment its owner chooses, not
alongside other work.

[guide]: ../AGENTS.md
[provider]: Provider.qml
