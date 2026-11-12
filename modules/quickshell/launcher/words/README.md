# The Launcher's Words

Every name, word and glyph the launcher shows or is found by, one file per
place. A thing's words go in the file named for the first part of its `in`
(`people.json` for a thing in `people:recent`), and a thing with no `in`
goes in `launcher.json`. `frame.json` holds the rest of the vocabulary: its
home, categories, commands, field words, the words meant to find more than
one thing, and the terminal's words.

Many entries because each place's words change with that place alone, so two
changes to different places never touch the same file or make each other's
checks run again. One words.json held them all until 12 Nov 2026, and
unrelated launcher changes kept going stale behind it.

[`../words.nix`][words] names each file and joins them into the one
vocabulary the shell and the daemons read, as `words.json`. A new place's
file is named there too. `each-places-words-are-a-file-of-their-own` fails
on a thing whose words sit in another place's file.

[words]: ../words.nix
