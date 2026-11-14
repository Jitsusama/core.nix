# The Shell's Files, by Place

Every QML and JavaScript file the shell links into its folder as it is, and
each script its views run from beside them (`opens-in-sessions.sh`), in
one list for each place the launcher names and for each surface beside it,
named as `checks/` names them: `heat.nix` for Heat, `clipboard.nix` for
Copies. Two places' changes then never share a file, where one list in
`home.nix` holding all of them made every change wait its turn.

A new file goes in the list of the one place whose view imports it. One that
several places' views import, or the launcher's own machinery (its prompt,
rows, keys and pacing), goes in `core.nix`, and the kinds that are no place's
have their own: `shell.nix`, `looks.nix`, `words.nix` and `command-line.nix`.
A place's own view, such as `launcher/Heat.qml`, isn't listed here: the
launcher's built-in kinds in `home.nix` link those.

Each file is a plain list of paths under `modules/quickshell/`, with no
module or option around it. `home.nix` joins them by name, never by reading
the folder, so a new place adds its file to that list and nothing else
changes there. A path two lists give stops the evaluation, since it would be
linked twice.
