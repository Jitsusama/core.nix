# Neovim's Probes

The Lua the checks run inside Neovim, started headless with the configuration an
account gets. Each probe is named for what it proves, and
[`../checks.nix`][checks] runs it as the check of that name with `neovim-`
before it, [`binds-every-key.lua`][binds] as `neovim-binds-every-key`. A probe
answers by writing to Neovim's output, and [`../probe.sh`][probe] fails the
check unless the answer is the one expected and Neovim complained of nothing.

Two differ:

- [`at-first-screen.lua`][first] is no check of its own: it runs the probe it's
  given once Neovim has entered, where a person meets it, rather than before.
- [`says-what-it-has-open.lua`][open] runs in a Neovim in a terminal of its own,
  which its check asks from outside what it has open, rather than through
  `probe.sh`.

[checks]: ../checks.nix
[probe]: ../probe.sh
[binds]: binds-every-key.lua
[first]: at-first-screen.lua
[open]: says-what-it-has-open.lua
