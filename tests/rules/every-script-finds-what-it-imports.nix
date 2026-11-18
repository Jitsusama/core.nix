# The check that every folder's scripts find what they import when mypy is
# handed them alone, as `nix fmt` hands it only what changed, for core and
# for any repository built on it, as `core.lib.everyScriptFindsWhatItImports
# pkgs`: every-script-finds-what-it-imports.py, beside this, says what counts.
# It first sees the rule fail a planted tree and pass it once what's imported
# is on mypy's path, as every-script-finds-what-it-imports.sh says.
{
  bash,
  lib,
  python3,
  runCommandLocal,
}:
let
  bare = python3.withPackages (python: [ python.mypy ]);
in
{
  # The repository's Python, as its flake sees it. Given only its .py and
  # .pyi files, the check runs again only when one of them changes.
  src,
  # The mypy the repository's treefmt runs, as its settings.formatter.mypy
  # holds it: its command, the options it's given, and the files it's
  # handed and those it's not.
  mypy,
  # What the repository's treefmt hands no formatter, its settings.excludes,
  # asked for rather than left empty, so no file treefmt skips is judged.
  excludes,
  name ? "every-script-finds-what-it-imports",
}:
let
  # Each glob is joined to its flag, so one beginning with '-' isn't read
  # as a flag of its own.
  selected =
    map (glob: "--include=${glob}") mypy.includes
    ++ map (glob: "--exclude=${glob}") (excludes ++ mypy.excludes or [ ]);
in
runCommandLocal name { } ''
  ${lib.getExe bash} ${./every-script-finds-what-it-imports.sh} \
    ${lib.getExe python3} ${./every-script-finds-what-it-imports.py} ${bare}/bin/mypy ${src} \
    ${lib.escapeShellArgs (
      selected
      ++ [
        "--"
        mypy.command
      ]
      ++ mypy.options or [ ]
    )}
  touch $out
''
