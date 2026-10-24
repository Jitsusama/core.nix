# Neovim's Keys

Neovim speaks the same language as every other layer's keys, described in
[the design][design] under The Keyboard. Space starts Neovim's own keys, and
after it come the words every layer shares:

| Keys                 | Does                                                |
| -------------------- | --------------------------------------------------- |
| Space Space          | find anything                                       |
| Space ?              | show every key                                      |
| Space w              | close the split, or the file when it's the last     |
| Space Tab            | go back to the file used last                       |
| Space Return         | open a terminal over the editor, or hide it         |
| Space and an arrow   | go to the split that way; with Shift, move it there |
| Space 1 to 9         | go to that tab                                      |

A letter after Space opens a group, and the group then Space finds among what
it holds: in a group of things, one of them (Space b Space an open file, Space
f Space a file), and in a group of things to do, one of its keys by what it
does (Space g Space, Space s Space), run as if pressed:

| Group   | Holds               | Group   | Holds                         |
| ------- | ------------------- | ------- | ----------------------------- |
| Space b | the open files      | Space k | the language server's folders |
| Space c | code                | Space s | search                        |
| Space e | problems            | Space t | tests                         |
| Space f | files               | Space T | tabs                          |
| Space g | git, and g h GitHub | Space u | looks                         |
| Space i | Neovim's insides    |         |                               |

A capital letter is that letter with Shift. [ and ] step to the one before
and after: [ d a problem, [ g a git change, [ t a test, [ b an open file.

The windows plugins open, the outline, the file browser and the problems
list, share the pickers' words:

| Keys             | Does                                    |
| ---------------- | --------------------------------------- |
| Return           | open it                                 |
| Ctrl+S, Ctrl+V   | open it below, or beside                |
| Ctrl+T           | open it in a tab, in the file browser   |
| q                | close the window                        |
| ?                | show the window's keys                  |
| Ctrl+L           | read it again                           |
| z and a letter   | fold, as anywhere in Neovim             |

While suggestions show, Tab and Shift+Tab choose among them, Ctrl+Y takes
one and Ctrl+E puts them away. The command line's are the same, for a
command and a search alike: with none showing, Tab suggests, and Ctrl+N and
Ctrl+P step through what was typed before.

## Finding a Key

Every key is in [keys.nix](keys.nix), with what it does in plain words, and
nowhere else. Home-manager writes it into `lua/bindings.lua`, which
[keys.lua](keys.lua) binds,
and into the map every layer shares, so either of these finds any key by
what it does:

- **Space ?** in Neovim, or pause after Space or a group's letter.
- **Super+Shift+/** anywhere, which searches every layer's keys at once.

## Adding a Key

Add it to keys.nix: a chord, what it does, and one of `lua`, `action`,
`command` or `keys`, or `plugin` with the plugin's name for one of its
actions, for a key in a plugin's own window, which its file hands to the
plugin in place of the plugin's own keys. A function too long for one line
goes in the plugin's file, registered under a name with
`require('keys').act(name, fn)`, and the key names it with `action`. A new
group needs its Space, which `neovim-finds-in-every-group` holds. The
`neovim-binds-every-key` check fails if a named action is missing, a key
isn't bound as keys.nix says, a plugin's window holds a key keys.nix doesn't
give it, or a file binds a key that says what it does without it being in
keys.nix.

[design]: ../../docs/design.md
