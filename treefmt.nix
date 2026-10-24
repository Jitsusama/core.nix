# How `nix fmt` formats and lints this repository, and what the formatting
# check holds it to: every linter core shares (treefmtModules.treefmt, from
# modules/treefmt/treefmt.nix), at its strictest, test code's included, and
# what only core has besides. ruff.toml and mypy.ini say how for Python,
# .luacheckrc for Lua, biome.jsonc for JavaScript and .rumdl.toml for
# Markdown.
{ pkgs, ... }:
{
  # The libraries core's own Python imports.
  jitsusama.mypy.packages =
    python:
    [
      python.dbus-fast
      python.fido2
      # The desktop test's caller of the portal.
      python.jeepney
      # The desktop test reads its photographs with Pillow.
      python.pillow
      # The desktop test's window named as Tuple's.
      python.xlib
    ]
    # The desktop test's touchscreen, which only Linux builds, so a Mac's
    # `nix fmt` still evaluates (mypy.ini says how it's read without it).
    ++ pkgs.lib.optional pkgs.stdenv.hostPlatform.isLinux python.evdev;

  # biome reads the JavaScript QML imports as the modules it means
  # (tests/qt-javascript-lint.sh).
  settings.formatter.qt-javascript-lint = {
    command = pkgs.lib.getExe (pkgs.callPackage ./tests/qt-javascript-lint.nix { });
    includes = [ "*.js" ];
    # The launcher is being redesigned, its matcher with it, which comes
    # under biome as it's rewritten.
    excludes = [ "modules/quickshell/Fuzzy.js" ];
  };
}
