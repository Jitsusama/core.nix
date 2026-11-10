# Modules

One folder per tool, named after the program or subsystem it configures,
holding its module for each system it runs on, its config files, its checks
and, for a program of its own, its code and tests. `flake.nix` names every
one, which is how to learn where a folder's modules are used.

Many entries because each folder is one tool, found by the tool's name:
grouping them by kind would give every tool a second name to remember, a
kind it often shares with another, and a path that changes whenever the
grouping does. A tool that grows parts keeps them in its own folder, as
`quickshell/` and `disk-guard/` do.
