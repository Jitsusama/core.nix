# How `nix fmt` formats and lints this repository, and what the formatting
# check holds it to: every linter core shares (treefmtModules.treefmt, from
# modules/treefmt/treefmt.nix), at its strictest, test code's included, and
# what only core has besides. ruff.toml and mypy.ini say how for Python,
# .luacheckrc for Lua, biome.jsonc for JavaScript and .rumdl.toml for
# Markdown.
{ pkgs, ... }:
let
  driver = pkgs.python3Packages.callPackage "${pkgs.path}/nixos/lib/test-driver" { };
  site = pkgs.python3.sitePackages;
  driverTypes = pkgs.python3Packages.toPythonModule (
    pkgs.runCommandLocal "test-driver-types" { } ''
      mkdir -p $out/${site}
      ln -s ${driver}/${site}/test_driver $out/${site}/
    ''
  );
in
{
  # A VM test's script names the test driver's types, which mypy reads from
  # the driver as it's built; it's a program rather than a library, so it's
  # given as one.
  jitsusama.mypy.packages = python: [
    python.dbus-fast
    python.fido2
    driverTypes
  ];

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
