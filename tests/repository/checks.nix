# The checks that read the repository as a whole, its notes, docs, names and
# patterns, which checks.nix gathers with every other folder's; README.md
# beside this says what each script here holds.
{
  formatterExcludes,
  lib,
  pkgs,
  self,
  ...
}:
{
  # Every surface takes its look from the theme, so no module but the theme's
  # names a colour or a face of its own: one that did would stay put when the
  # theme changes. Colours are caught as hex or CSS functions, faces by the
  # families anything here installs or falls back to
  # (tests/repository/nothing-names-its-own-look.sh).
  nothing-names-its-own-look =
    let
      colours = [
        "#[0-9A-Fa-f]{6}([0-9A-Fa-f]{2})?\\b"
        "\\b(rgba?|hsla?)\\("
      ];
      faces = [
        "Monaspice"
        "Monaspace"
        "Mona Sans"
        "Hubot Sans"
        "Fira Code"
        "Fira Mono"
        "Victor Mono"
        "JetBrains Mono"
        "Nerd Font"
        "Adwaita Sans"
        "Adwaita Mono"
        "DejaVu"
        "Noto Sans"
        "Noto Serif"
        "Noto Naskh"
        "Terminus"
      ];
      pattern = lib.concatStringsSep "|" (colours ++ [ "\\b(${lib.concatStringsSep "|" faces})\\b" ]);
    in
    pkgs.runCommandLocal "nothing-names-its-own-look" { nativeBuildInputs = [ pkgs.ripgrep ]; } ''
      ${lib.getExe pkgs.bash} ${./nothing-names-its-own-look.sh} ${../..} \
        ${lib.escapeShellArg pattern}
      touch $out
    '';

  # Every surface draws its marks from the theme's table (modules/theme/
  # glyphs.nix) by name, so no module but the theme's spells one of the
  # table's code points itself: a surface that did would keep its mark when
  # the table changes, and could drift from the mark the same idea wears
  # elsewhere. Tests spell what they expect to see, which is their job, so no
  # tests folder is read, at the top or beside its module
  # (tests/repository/nothing-spells-a-mark.sh).
  nothing-spells-a-mark =
    pkgs.runCommandLocal "nothing-spells-a-mark" { nativeBuildInputs = [ pkgs.ripgrep ]; }
      ''
        ${lib.getExe pkgs.bash} ${./nothing-spells-a-mark.sh} ${../..}
        touch $out
      '';

  # Quickshell's JavaScript lacks some of the language's newer methods, which
  # qmllint lets through and the shell fails on only as it runs, so the
  # shell's code calls none of them, in Quickshell's module and in every
  # module that gives the shell a file. Spotify's Escape script runs in
  # Spotify's own browser, which has them, so it alone is left out
  # (tests/repository/the-desktop-speaks-its-javascript.sh).
  the-desktop-speaks-its-javascript =
    pkgs.runCommandLocal "the-desktop-speaks-its-javascript" { nativeBuildInputs = [ pkgs.ripgrep ]; }
      ''
        ${lib.getExe pkgs.bash} ${./the-desktop-speaks-its-javascript.sh} ${../..}
        touch $out
      '';

  # Every folder says what goes in it, so a reader reaches any file from the
  # map by names alone: in a README, the opening comment of its entry file
  # or of a file beside it of its name, a line of its parent's README
  # opening with its name, or a line of the top README's or AGENTS.md's
  # map. The folders below are still waiting for their note, and the check
  # fails once one has it or is gone, so the list only shrinks.
  every-folder-says-what-it-holds =
    let
      waiting = [
        # The desk keyboard's layout, which firmware.nix describes. Its
        # folder's hash is the board's serial, so a note in it would change
        # the firmware and need a reflash; it takes one with the next
        # layout change.
        "modules/ergodox/keymap"
      ];
    in
    self.lib.everyFolderSaysWhatItHolds pkgs {
      src = ../..;
      inherit waiting;
    };

  # A doc names only what's there, so a move or a rename that leaves a guide
  # pointing at nothing fails here rather than misleading the next reader:
  # every relative link, and every backticked path from a top folder. Those
  # below name something deliberately absent, and fail once they're gone.
  docs-name-what-exists = pkgs.callPackage ./docs-name-what-exists.nix { } {
    src = ../..;
    exempt = [
      # A module a reader would add, as an example.
      "docs/modules.md modules/jq/checks.nix"
      # A file in the kernel's own source tree.
      "docs/hardware.md tools/power/x86/turbostat/Makefile"
    ];
  };

  # core.nix knows nothing about what builds on it: no file names a machine, a
  # machine repository or an employer, so the repositories that use core name
  # it and never the other way round. Every file is read but the checks.nix
  # files, as this one has to name the words, and the lock file, which only
  # names pins.
  stands-alone =
    let
      outside = [
        "shopify"
        "/opt/dev"
        "gitstream"
        "devx"
        "minerva"
        "cloudflare"
        "dotfiles"
        "joel-gerber"
        "joel.gerber"
        "grrbrr"
        "monorepo"
        "work repository"
        "work layer"
        "personal repository"
        "personal layer"
        "optimus"
        "penelope"
        "methuselah"
        "equilibrius"
      ];
    in
    pkgs.runCommandLocal "stands-alone" { nativeBuildInputs = [ pkgs.ripgrep ]; } ''
      cd ${../..}
      if rg --ignore-case --fixed-strings --line-number --no-heading --hidden \
        --glob '!checks.nix' --glob '!flake.lock' \
        ${lib.concatMapStringsSep " " (word: "-e ${lib.escapeShellArg word}") outside} .; then
        echo "The lines above name what builds on core.nix; they belong where it is built."
        exit 1
      fi
      touch $out
    '';

  # The glossary gives each thing one word and each word one thing within
  # its context, and writes no name it replaces
  # (tests/repository/glossary.py), printing each word to settle.
  the-glossary-names-each-thing-once =
    pkgs.runCommandLocal "the-glossary-names-each-thing-once" { }
      ''
        ${lib.getExe pkgs.python3} ${./glossary.py} ${../../docs/glossary.md}
        touch $out
      '';

  # No VM test grows past twenty subtests, where docs/testing.md says one
  # splits by area (tests/repository/every-vm-test-keeps-to-the-pyramid.py).
  # Those past it today are listed with their counts, which may only
  # fall, and each comes off once it's within the limit, so the list only
  # shrinks. It reads the VM tests' files alone, so no other change reruns it.
  every-vm-test-keeps-to-the-pyramid =
    let
      over = [
        # One machine's story, kept whole since a split would replay it
        # (docs/testing.md, Earning a Check).
        "tests/vm/guide/guide-install-vm-test.py 26"
      ];
      tests = lib.fileset.toSource {
        root = ../..;
        fileset = lib.fileset.fileFilter (file: lib.hasSuffix "-vm-test.py" file.name) ../..;
      };
    in
    pkgs.runCommandLocal "every-vm-test-keeps-to-the-pyramid" { } ''
      ${lib.getExe pkgs.python3} ${./every-vm-test-keeps-to-the-pyramid.py} ${tests} \
        ${pkgs.writeText "over" (lib.concatLines over)}
      touch $out
    '';

  # Every path pattern merge-net's must-run.txt, ruff.toml's per-file
  # ignores and what treefmt leaves unread give still names a file
  # (tests/repository/every-pattern-names-a-file.py), since none of the tools
  # says when one matches nothing, and a file moved from under one would
  # quietly drop its VM tests from the net or its ignores from ruff, and
  # the next file to take its name would go unlinted. It's given the files'
  # paths, not what they hold, so only a file added, moved or removed runs it
  # again.
  every-pattern-names-a-file =
    let
      paths = map (file: lib.removePrefix "./" (lib.path.removePrefix ../.. file)) (
        lib.fileset.toList ../..
      );
    in
    pkgs.runCommandLocal "every-pattern-names-a-file" { } ''
      ${lib.getExe pkgs.python3} ${./every-pattern-names-a-file.py} \
        ${../../tools/merge-net/must-run.txt} ${../../ruff.toml} \
        ${pkgs.writeText "unread" (lib.concatLines formatterExcludes)} \
        ${pkgs.writeText "paths" (lib.concatLines paths)}
      touch $out
    '';
}
