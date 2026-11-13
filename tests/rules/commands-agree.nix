# The check that every command a repository's crates and script wrappers
# build keeps the convention docs/commands.md sets, for core and for any repository built on
# it, as `core.lib.commandsAgree pkgs`: commands-agree.py, beside this, says
# what each point holds.
{
  lib,
  python3,
  runCommandLocal,
  writeText,
}:
{
  # The repository's tree, as its flake sees it, whose crates and Nix
  # wrappers build the commands.
  src,
  # The commands on the terminal kit, built, by name: the check runs each
  # for its help, so every one on the kit is given here.
  commands ? { },
  # The points of the convention each command broke before the rule, by its
  # name, while it comes to agree. The check fails on a command breaking a
  # point it doesn't wait on, and on one waiting on a point it keeps.
  waiting ? { },
  # What each command given here answers a person from a machine's state,
  # by its name: a list of views, each the words after the program, as
  # `args`, and the folder of stand-in state it reads, as `state`, whose
  # config, state, cache, data and runtime folders are its XDG folders.
  # A view may set variables of its own, as `env`, and its words and
  # variables, and its state's .in files, may name the folder its state is
  # put in as @HERE@. Its .in files' times are @NOW-N@ in seconds since
  # the epoch, or @ISO-N@ in RFC 3339. Steps to run first, in the same
  # state, are `first`, a list of words each. It ends with 0, or the
  # `exits` it names, as 1 where it says something differs.
  # Each is set at the widths its help is, and has to fit as help does.
  views ? { },
  name ? "commands-agree",
}:
let
  runs = lib.mapAttrs (program: package: lib.getExe' package program) commands;
in
runCommandLocal name { } ''
  ${lib.getExe python3} ${./commands-agree.py} ${src} \
    ${writeText "runs.json" (builtins.toJSON runs)} \
    ${writeText "waiting.json" (builtins.toJSON waiting)} \
    ${writeText "views.json" (builtins.toJSON views)}
  touch $out
''
