# Quickshell's Checks

The shell's checks, one file for each place the launcher names and for each
surface beside it, named as the launcher names it: `heat.nix` for Heat,
`clipboard.nix` for Copies. Two places' changes then never share a file,
where one `checks.nix` holding all of them made every change wait its turn.

A check goes with the one place whose QML it copies. One copying several
places', or the launcher's own machinery (its prompt, rows, keys, pacing and
the launcher on niri), goes in `core.nix`, and the kinds that are no place's
have their own: `shell.nix` for how the shell is built and linted,
`looks.nix`, `words.nix`, `terminal.nix` and `command-line.nix`.

`helpers.nix` holds what more than one file uses: stand-ins for the shell's
singletons, the shell as the account gets it, and the harnesses that run it
offscreen or on niri. Each file takes what it needs of these by name, beside
the folder's own arguments.

A check copies the shell's own files with `copied` from `helpers.nix`,
naming only the ones it uses. `imports.nix` says what each file imports of
the shell's, and `copied` brings those with it, so a file gaining an import
changes its row there and no check. A file whose imports differ from its row
fails every check that copies it, naming the file, so the table can't drift.

The folder's `checks.nix` lists every file here by name, never by reading
the folder, so a new place adds its file to that list and nothing else
changes there. A check name two files give stops the evaluation.
