# The check that every command a repository's crates build keeps the
# convention docs/commands.md sets, for core and for any repository built on
# it, as `core.lib.commandsAgree pkgs`: commands-agree.py, beside this, says
# what each point holds.
{
  lib,
  python3,
  runCommandLocal,
  writeText,
}:
{
  # The repository's tree, as its flake sees it, whose crates build the
  # commands.
  src,
  # The commands on the terminal kit, built, by name: the check runs each
  # for its help, so every one on the kit is given here.
  commands ? { },
  # The points of the convention each command broke before the rule, by its
  # name, while it comes to agree. The check fails on a command breaking a
  # point it doesn't wait on, and on one waiting on a point it keeps.
  waiting ? { },
  name ? "commands-agree",
}:
let
  runs = lib.mapAttrs (program: package: lib.getExe' package program) commands;
in
runCommandLocal name { } ''
  ${lib.getExe python3} ${./commands-agree.py} ${src} \
    ${writeText "runs.json" (builtins.toJSON runs)} \
    ${writeText "waiting.json" (builtins.toJSON waiting)}
  touch $out
''
