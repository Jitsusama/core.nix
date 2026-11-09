# How `nix fmt` formats and lints this repository, and what the formatting
# check holds it to: every linter core shares (treefmtModules.treefmt, from
# modules/treefmt/treefmt.nix), at its strictest, test code's included, and
# what only core has besides. ruff.toml and mypy.ini say how for Python,
# .luacheckrc for Lua, biome.jsonc for JavaScript and TypeScript and
# .rumdl.toml for Markdown.
{ pkgs, ... }:
{
  # A file no formatter reads stops `nix fmt` and the formatting check, so a
  # new kind of file can't slip past every linter unseen. What no linter
  # reads yet is named below by kind, each with its reason; giving a kind
  # its linter takes its line off.
  settings.on-unmatched = "fatal";

  settings.excludes = [
    # The shell's QML is read by quickshell-passes-qmllint as the account
    # installs it; the QML cases in tests/ run in Qt's own engine, unlinted.
    "*.qml"
    # Cargo reads each crate's manifest as the crate builds, refusing a
    # malformed one; no TOML linter reads it, nor the other TOML, yet.
    "*.toml"
    # JSON, read by the programs and tests that load it; none formats it yet.
    "*.json"
    "*.jsonc"
    "*.jsonl"
    # What programs and devices answered, recorded for the tests to compare
    # byte for byte, and the stand-in keys and signatures they sign with.
    "modules/eero/src/views/golden/*"
    "modules/guide/fixtures/*"
    "modules/signing/tests/*.sig"
    "tests/inkscape-recent.xbel"
    "*.pem"
    "tests/update-to-main-signer"
    # C and C++ built by the kernel's, QMK's and CMake's own builds, which
    # stop on an error but don't lint.
    "*.c"
    "*.cpp"
    "*.h"
    "*/Kbuild"
    "*/CMakeLists.txt"
    "*/rules.mk"
    # Settings in a program's own format, read by the program they set.
    "*.conf"
    "*/kernel/*.config"
    "*.ini"
    "*.kdl"
    "*.quirk"
    "*.script"
    "*.svg"
    "*.xml"
    "*.yml"
    ".luacheckrc"
    "modules/ghostty/config"
    "modules/gimp/gimprc"
    "modules/gimp/shortcutsrc"
    # polkit's rules are JavaScript and udev's their own lines; the nft,
    # bpftrace and jq programs are read only by their own tools as they run.
    "*.rules"
    "*.nft"
    "*.bt"
    "*.jq"
    # Word lists, one per line: a dictionary, glyphs, paths and check names.
    "*.txt"
    "*.add"
    # The licences that came with code taken from another project, kept
    # beside it word for word.
    "*/LICENSE.*"
    # The launcher's matcher, being redesigned, comes under biome as it's
    # rewritten (as qt-javascript-lint says).
    "modules/quickshell/Fuzzy.js"
  ];

  # A stand-in program for a test, named as the program it stands in for,
  # so its kind isn't in its name.
  settings.formatter.shellcheck.includes = [ "modules/quickshell/in-a-terminal/tests/qs" ];

  # The libraries core's own Python imports.
  jitsusama.mypy.packages =
    python:
    [
      python.dbus-fast
      python.fido2
      # The desktop test's caller of the portal.
      python.jeepney
      # The noise-cancelled microphone's export, and the stand-in it loads.
      python.numpy
      # The desktop test reads its shots with Pillow.
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
    # under biome as it's rewritten. Chrome's extension isn't QML's.
    excludes = [
      "modules/quickshell/Fuzzy.js"
      "modules/chrome/open-tabs/extension/*.js"
    ];
  };

  # Chrome's open-tabs extension, read by biome as the browser runs it.
  settings.formatter.browser-javascript-lint = {
    command = pkgs.lib.getExe pkgs.biome;
    options = [
      "lint"
      "--error-on-warnings"
      "--max-diagnostics=none"
      "--config-path=biome.jsonc"
    ];
    includes = [ "modules/chrome/open-tabs/extension/*.js" ];
  };

  # The web kit's CSS, read by biome as a browser reads it.
  settings.formatter.css-lint = {
    command = pkgs.lib.getExe pkgs.biome;
    options = [
      "lint"
      "--error-on-warnings"
      "--max-diagnostics=none"
      "--config-path=biome.jsonc"
    ];
    includes = [ "*.css" ];
  };

  # biome formats and lints Node's JavaScript, the .mjs tests and pi's
  # TypeScript extensions, as biome.jsonc's overrides for them say.
  settings.formatter.biome = {
    command = pkgs.lib.getExe pkgs.biome;
    options = [
      "check"
      "--write"
      "--error-on-warnings"
      "--max-diagnostics=none"
      "--config-path=biome.jsonc"
    ];
    includes = [
      "*.mjs"
      "*.ts"
    ];
  };
}
