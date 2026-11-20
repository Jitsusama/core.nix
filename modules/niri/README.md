# niri

The compositor Joel's desktop runs on: `nixos.nix` installs it with our own
patches and runs it as the session, with `niri-deadline.sh` to try its
render deadline by turns. `home.nix` gives the account its config:
`config.kdl` says how niri behaves and `binds.nix` what each key does.
`checks.nix` proves them on a niri nested in the build sandbox, with the
stand-ins in `tests/`. xwayland-satellite's package and script sit beside
it, the program niri starts for X11 programs.

Each `.patch` but three is one change to niri, said in its own header, and
`nixos.nix` lists them in the order they apply, with a paragraph for each
saying why it's there. The three, `untitled-instance.patch`,
`newer-backend.patch` and `flush-when-full.patch`, change
xwayland-satellite, and `xwayland-satellite.nix` applies them. Many entries
because each patch keeps its own file beside the list that applies it: a
patch is read, rebased and dropped on its own, and the lists name every
one, so a folder of them would only add a level each is found through.
