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
  # module that gives the shell a file. Spotify's keys script and the
  # mycelium map's drawing run in a browser, which has them, so they alone
  # are left out (tests/repository/the-desktop-speaks-its-javascript.sh).
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
  # names pins. The work's own projects, people and places are on the list
  # too, so a study of a site Joel is proud of keeps to its design lessons
  # here and leaves its subject where it was built.
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
        "tobi"
        "worldtree"
        "part deux"
        "world-in-a-bottle"
        "the-one-log"
        "omni-refs-4tw"
        "developer & ai"
        "#inv-"
        "aquifer"
        "acceluary"
        "deploy v3"
        "burke libbey"
        "bruckmayer"
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

  # No hardware address here could be a real device's, since one copied
  # beside the network a test names says where a machine has been
  # (tests/repository/every-hardware-address-is-made-up.sh). Those below
  # are kept on purpose: each has a real maker's prefix, as a vendor is
  # named from or a card is read like, under a device part plainly made up.
  # A made-up tree shows a copied address is refused and these aren't.
  every-hardware-address-is-made-up =
    let
      script = "${lib.getExe pkgs.bash} ${./every-hardware-address-is-made-up.sh}";
      kept = pkgs.writeText "kept" (
        lib.concatLines [
          # Headphones in Bluetooth's links, by their maker's prefix.
          "00:1b:66:aa:bb:cc"
          # A wired card the shell names by its maker.
          "00:1b:21:aa:bb:cc"
          # A headset's sound devices, as PipeWire names them from it.
          "00:11:22:33:44:55"
          # A cable's card as the shell reads it, and the address it was made with.
          "3c:22:fb:00:00:01"
          # The launcher's example of a vendor named from an address.
          "3c:22:fb:12:34:56"
          # A trusted cable, the option's example and its check.
          "44:67:52:00:00:01"
          # The address a cable was made with, beside the one it's given.
          "8c:16:45:00:00:02"
          # A Wi-Fi radio's own address, read from sysfs.
          "8c:f8:c5:12:34:5f"
          # A Bluetooth mouse's radio, as its HID device says it.
          "b4:8c:9d:00:00:01"
          # A Bluetooth keyboard, as its HID device says it.
          "f4:4e:fd:00:11:22"
        ]
      );
      madeUp = {
        said = pkgs.writeText "said" ''
          ./copied.txt:1:00:00:00:00:00:01
          The addresses above could be a real device's. Write a made-up one, as
          02:00:00:00:a0:00, or RFC 7042's 00:00:5e:00:53:01; an address that needs
          a vendor's real prefix is kept on purpose in tests/repository/checks.nix.
        '';
      };
    in
    pkgs.runCommandLocal "every-hardware-address-is-made-up" { nativeBuildInputs = [ pkgs.ripgrep ]; }
      ''
        ${script} ${../..} ${kept}
        mkdir made-up
        # A maker's address, then made-up ones of each kind, seven octets, and
        # a kept one in upper case.
        echo '00:00:00:00:00:01' >made-up/copied.txt
        echo '02:00:00:00:a0:00 52:54:00:12:34:56 ff:ff:ff:ff:ff:ff 01:00:5e:00:00:fb' \
          '00:00:5E:00:53:01 00:00:00:00:00:00 00:00:00:00:00:01:01 00:1B:66:AA:BB:CC' \
          >made-up/made-up.txt
        if ${script} made-up ${kept} 2>said; then
          echo "It passed a maker's address no one kept." >&2; exit 1
        fi
        diff ${madeUp.said} said
        touch $out
      '';

  # Every module core offers is brought into an example machine, by a role,
  # a hardware model, the example itself or another module, so it meets the
  # others there and two that clash fail the example, not a machine
  # (tests/repository/every-module-meets-the-others.sh). A made-up module
  # whose own folder alone names it shows that folder doesn't count.
  every-module-meets-the-others =
    let
      script = "${lib.getExe pkgs.bash} ${./every-module-meets-the-others.sh}";
      # foo, which only its own folder names, and bar, which foo's names.
      madeUp = {
        outputs = pkgs.writeText "made-up" (
          lib.concatLines (
            map (name: "nixosModules ${name}") [
              "foo"
              "bar"
            ]
          )
        );
        said = pkgs.writeText "said" ''
          No role, hardware, example or other module brings these, so they meet
          the rest only on a machine; import each in an example (examples/):
            nixosModules.foo
        '';
      };
      offered = system: map (name: "${system} ${name}") (lib.attrNames self.${system});
      outputs = lib.concatMap offered [
        "homeModules"
        "nixosModules"
        "darwinModules"
      ];
    in
    pkgs.runCommandLocal "every-module-meets-the-others" { nativeBuildInputs = [ pkgs.ripgrep ]; } ''
      ${script} ${../..} ${pkgs.writeText "outputs" (lib.concatLines outputs)}
      mkdir -p made-up/roles made-up/hardware made-up/examples made-up/modules/foo
      touch made-up/flake.nix; echo '"../foo/a.nix" "../bar/a.nix"' >made-up/modules/foo/b.nix
      if ${script} made-up ${madeUp.outputs} 2>said; then
        echo "It counted a module that only its own folder names as brought." >&2; exit 1
      fi
      diff ${madeUp.said} said
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
  # ignores, what treefmt leaves unread and what this folder's scripts and
  # checks tell ripgrep to leave out give still names a file
  # (tests/repository/every-pattern-names-a-file.py), since none of the tools
  # says when one matches nothing, and a file moved from under one would
  # quietly drop its VM tests from the net or its ignores from ruff, the
  # next file to take its name would go unlinted, and a script would read
  # the file it meant to leave out. It's given the files' paths, not what
  # they hold, the checks' names, which an area naming a check matches, and
  # each file's ripgrep exclusions, read from it here, so only a file added,
  # moved or removed, or an exclusion changed, runs it again. It reads
  # those against the root, which each script searches from, so it can't
  # see a script searching from somewhere else, nor a pattern a shell
  # variable fills in, nor one the shell or Nix fills in after its `!`,
  # though it reads a quoted one's backtick as written, nor a flag joined to
  # others, as -lg. A whole glob a `$(` or Nix gives after the flag on a
  # line running rg, or a backtick after a space, `=` or quote, it can't
  # read, and says so. It reads each line's quotes afresh, so a quote left
  # open across lines can hide one, and it reads a pattern glued to a
  # quoted part opening with a space, as '!a'" b", as ending at the space.
  # A made-up tree and script show it reads each spelling, says each way a
  # pattern names nothing, and no way one names a file.
  every-pattern-names-a-file =
    let
      check = "${lib.getExe pkgs.python3} ${./every-pattern-names-a-file.py}";
      relative = file: lib.removePrefix "./" (lib.path.removePrefix ../.. file);
      paths = map relative (lib.fileset.toList ../..);
      # Each `-g`, `--glob` or `--iglob` starting a word and given a `!`,
      # quoted or not, past a line that's a comment, since a comment may quote
      # one, and past a `#` closing a line outside quotes, read as the shell
      # reads them. A `\` then joins a line to the next, and so does a flag
      # ending one, as in a list nixfmt has split. A pattern runs to its own
      # closing quote, or a bare one to a space, a quote or what ends a
      # command, and must end its word there. A line holding more flags given
      # a `!` than it reads, or on a line running rg, by name, by a variable or
      # by its store path, a `$(`, a backtick after a space, `=` or quote, or
      # in a .nix file a `${`, gives the line with no flag, so the check says
      # it can't read it rather than passing it by. The made-up script below
      # splices in its `!`, so this file's own reading finds none, and its
      # `$(`, `${` and backticks, which that reading takes off.
      bang = "!";
      run = "$(";
      tick = "`";
      brace = "\${";
      flag = "(-g|--glob|--iglob)";
      start = "[ '\"(=]";
      single = "'${bang}([^']*)'";
      double = "\"${bang}([^\"\\\\]*)\\\\?\"";
      bare = "${bang}([^'\"`\\\\ ;)|&]*)";
      stop = "[] ;)|&]";
      exclusion = "${flag}['\"]?[ =]*\\\\?(${single}|${double}|${bare})(${stop}|['\"`]+${stop})";
      given = "${start}${flag}['\"]?[ =]*\\\\?['\"]?";
      ending = ".*${given}";
      leftOut =
        name: text:
        let
          code = lib.filter (line: !lib.hasPrefix "#" (lib.trim line)) (lib.splitString "\n" text);
          # Where the `#` opening a line's comment is: one after what ends a
          # word and outside quotes, a `\` escaping what follows it but in
          # single quotes. A `${` holds Nix's names, whose primes quote nothing,
          # and its strings.
          comment =
            line:
            let
              step =
                state: c:
                let
                  quote = if state.quotes == [ ] then null else lib.head state.quotes;
                  next = state // {
                    at = state.at + 1;
                    before = c;
                    escaped = false;
                  };
                  opened = next // {
                    quotes = [ c ] ++ state.quotes;
                  };
                  closed = next // {
                    quotes = lib.tail state.quotes;
                  };
                in
                if state.cut != null then
                  state
                else if state.escaped then
                  next // { before = "\\"; }
                else if c == "\\" && quote != "'" then
                  next // { escaped = true; }
                else if quote == "'" then
                  if c == "'" then closed else next
                else if (c == "(" || c == "{") && state.before == "$" then
                  opened
                else if quote == "\"" then
                  if c == "\"" then closed else next
                else if quote == "{" then
                  if c == "}" then
                    closed
                  else if c == "\"" then
                    opened
                  else
                    next
                else if c == "'" || c == "\"" then
                  opened
                else if c == ")" && quote == "(" then
                  closed
                else if c == "#" && lib.elem state.before breaks then
                  state // { cut = state.at; }
                else
                  next;
              # What ends a word before a `#`, a `\` having escaped none.
              breaks = [
                " "
                ";"
                "&"
                "|"
                "("
                ")"
              ];
              begun = {
                at = 0;
                before = " ";
                quotes = [ ];
                escaped = false;
                cut = null;
              };
            in
            (lib.foldl' step begun (lib.stringToCharacters line)).cut;
          uncommented =
            line:
            let
              flat = builtins.replaceStrings [ "\t" ] [ " " ] line;
              at = if lib.hasInfix "#" flat then comment flat else null;
            in
            lib.trim (if at == null then flat else builtins.substring 0 at flat);
          ended = lib.concatMapStrings (line: "${uncommented line}\n") code;
          joined = builtins.replaceStrings [ " \\\n" "\\\n" ] [ " " " " ] ended;
          lines = lib.splitString "\n" joined;
          ends = map (line: builtins.match ending " ${line}" != null) lines;
          # A line a flag ending the one before carried is read only there.
          carries = i: lib.elemAt ends i && !lib.elemAt carried i;
          carried = lib.genList (i: i > 0 && carries (i - 1)) (lib.length lines);
          next = i: if i + 1 < lib.length lines then " ${lib.elemAt lines (i + 1)}" else "";
          read = i: line: if lib.elemAt ends i then "${line}${next i}" else line;
          kept = lib.concatLists (
            lib.imap0 (i: line: lib.optional (!lib.elemAt carried i) (read i line)) lines
          );
          parts = line: builtins.split exclusion " ${line} ";
          pattern = match: lib.findFirst (group: group != null) "" (lib.sublist 2 3 match);
          row = match: lib.optional (pattern match != "") "${lib.head match}\t${pattern match}";
          # A match counts only where its flag starts a word, or just past the
          # one before it, whose own end used the space between.
          starts = before: before == "" || builtins.match ".*${start}" before != null;
          counted =
            all: i: part:
            lib.optionals (lib.isList part && starts (lib.elemAt all (i - 1))) (row part);
          found = line: lib.concatLists (lib.imap0 (counted (parts line)) (parts line));
          times = regex: line: lib.count lib.isList (builtins.split regex " ${line} ");
          named = "([ ;|&(`/'\"=$\{]rg|ripgrep}|\"rg\"})[}'\"]*";
          runs = line: builtins.match ".*${named} .*" " ${line} " != null;
          # A `${` is a splice only in a .nix file, a .sh file's being the
          # shell's own variable, and this file's reading of itself meets its
          # own splices, which it takes off.
          shell = "\\$\\(|[ =\"']+`";
          fills = if lib.hasSuffix ".nix" name then "${shell}|\\$\\{" else shell;
          filled = "${given}(${fills})";
          own = "${given}\\$\\{(bang|run|tick|brace)}";
          splices = line: if name == relative ./checks.nix then times own line else 0;
          made = line: if runs line then times filled line - splices line else 0;
          flags = line: times "${given}${bang}" line + made line;
          unread = line: flags line > lib.length (found line);
          said = line: if unread line then [ "\t${line}" ] else found line;
        in
        map (fields: "${name}\t${fields}") (lib.concatMap said kept);
      read = file: leftOut (relative file) (builtins.readFile file);
      here = lib.concatMap read sources;
      sources = lib.fileset.toList (
        lib.fileset.fileFilter (file: file.hasExt "sh" || file.hasExt "nix") ./.
      );
      stale =
        pattern:
        "t.nix tells ripgrep to leave out ${pattern}, which matches no file: "
        + "point it at where the file is now, or take it off.";
      unreadable =
        line:
        "t.nix holds a ripgrep glob this check can't read, in ${line}: "
        + "write each pattern given after the flag out in full.";
      madeUp = {
        paths = [
          "flake.nix"
          "README.md"
          "modules/spotify/speaks-vims-keys.js"
          "modules/theme/glyphs.nix"
          "modules/a/tests/case.py"
          "tests/x.py"
          "y/x"
        ];
        # Each spelling ripgrep takes, and some it takes that this can't read.
        # Kept in Nix because it's text the check reads and never runs, its
        # splices keeping its exclusions out of this file's own reading.
        script = ''
          # rg --glob '${bang}gone.js', which a comment only quotes, ending in \
          rg --glob '${bang}gone/c.js'
          rg --glob '${bang}modules/spotify/backs-out.js' --glob "${bang}modules/*.js"
          rg -g ${bang}modules/*/speaks-vims-keys.js -g'${bang}**/tests/**'
          rg --glob=${bang}modules/theme/**/glyphs.nix --glob '${bang}/flake.nix'
          rg --glob '${bang}tests/' --glob '${bang}x/' --glob '${bang}x' --glob '${bang}a/'
          rg --iglob '${bang}readme.MD' --glob '${bang}readme.MD' --glob '${bang}{a,b}.js'
          rg --glob "${bang}$name/**" --glob '${bang}spotify/speaks-vims-keys.js'
          rg --glob '${bang}/x' --glob '${bang}[^z]lake.nix' $(rg -l -g ${bang}y/x)
          rg -g ${bang}tests/x.py; rg --glob \${bang}flake.nix | rg --glob '${bang}modules/a dir/**'
          lib.escapeShellArgs [ "--glob" "${bang}README.md" ]
          rg --glob \
            '${bang}gone/b.js'
          rg --glob '${bang}README.md' --glob "${run}printf '${bang}%s' x)"
          rg --glob '${bang}'modules/x.js
          lib.escapeShellArgs [
            "--glob"
            "*.nix"
          ]
          rg --glob '${bang}x' modules # once --glob '${bang}gone.js'
          rg -g '*.sh' '^#${bang}' modules; [ "$n" -gt 0 ] && ${bang} cmp a b
          rg --glob \"${bang}README.md\"
          rg --glob '${bang}modules/spotify/'backs-out.js
          lib.escapeShellArgs [
            "--glob"
            "${bang}gone/d.js"
          ]
          echo usage: rg -g
          rg -g '${bang}gone/e.js'
          rg --glob '${bang}x' # once, so not \
          rg --glob '${bang}gone/f.js'
          rg -e ' #' --glob '${bang}gone/g.js'
          rg --glob '${bang}x'${"\t"}# once --glob '${bang}gone.js'
          [ "--glob=${bang}gone/h.js" ]
          script = "rg --glob '${bang}README.md'";
          rg --glob '${bang}README.md" x'
          x-g '${bang}README.md' && rg --glob "${run}f)"
          rg --glob "${tick}printf '${bang}%s' gone${tick}"
          x-g '${bang}gone/n.js'
          rg -e " #" --glob '${bang}gone/i.js'
          x = "rg -e \" #\" --glob '${bang}gone/k.js'";
          rg -e "don't" -e ' #' --glob '${bang}gone/l.js'
          echo -g
          echo -g
          rg -g '${bang}gone/j.js'
          install -g "${run}id -gn)" a b
          rg --glob '${bang}modules/a  dir/**'
          rg -g'${bang}modules/a/'b.js
          rg --glob '${bang}modules/a/'"case.py"
          x="$(rg -e "a #" --glob '${bang}gone/z.js')"
          rg -g '${bang}gone/w.js';# was -g '${bang}gone.js'
          rg -e a\ #b --glob '${bang}gone/y.js'
          rg -e \' --glob '${bang}x' # --glob '${bang}gone.js'
          rg -e '\' --glob '${bang}x' # --glob '${bang}gone.js'
          ''${lib.getExe pkgs.ripgrep} --glob "${run}f)"
          /bin/rg --glob "${run}f)"
          "''${lib.getExe pkgs.ripgrep}" --glob "${run}f)"
          script = "rg --glob \"${run}f)\"";
          rg --glob ${brace}lib.escapeShellArg "${bang}gone.js"}
          "$rg" --glob "${run}f)"
          ''${lib.getExe' pkgs.ripgrep "rg"} --glob "${run}f)"
          ''${rg} --glob "${run}f)"
          ''${lib.getExe' pkgs.ripgrep "rg"} --glob '${bang}gone/v.js' # was --glob '${bang}gone.js'
          message = "pass `rg -g` a glob";
        '';
        # What it says, in the script's order. The comments, what's included,
        # a flag not starting a word, another command's flag and a code span
        # say nothing.
        said =
          # Past a comment line ending in a `\`, the file gone, a * that stops
          # at a slash, a folder that's only a file, a name in another case.
          map stale [
            "gone/c.js"
            "modules/spotify/backs-out.js"
            "modules/*.js"
            "x/"
            "readme.MD"
          ]
          ++ [
            (
              "t.nix tells ripgrep to leave out {a,b}.js, whose braces this check can't read: "
              + "give each choice a glob of its own."
            )
          ]
          # A path that holds a slash and so starts at the root, one a leading
          # slash anchors there, a quoted one holding a space, and one a `\`
          # carries to the next line.
          ++ map stale [
            "spotify/speaks-vims-keys.js"
            "/x"
            "modules/a dir/**"
            "gone/b.js"
          ]
          # A `$(`, an empty pattern and a quoted one glued to more.
          ++ map unreadable [
            "rg --glob '${bang}README.md' --glob \"${run}printf '${bang}%s' x)\""
            "rg --glob '${bang}'modules/x.js"
            "rg --glob '${bang}modules/spotify/'backs-out.js"
          ]
          # A split list carried, a flag ending the line before carried and
          # said once, past a comment closing a line in a `\`, past a ` #` in
          # quotes, a whole quoted word, and quotes that don't match.
          ++ map stale [
            "gone/d.js"
            "gone/e.js"
            "gone/f.js"
            "gone/g.js"
            "gone/h.js"
            "README.md\" x"
          ]
          # A `$(` beside a flag not starting a word, and a backtick.
          ++ map unreadable [
            "x-g '${bang}README.md' && rg --glob \"${run}f)\""
            "rg --glob \"${tick}printf '${bang}%s' gone${tick}\""
          ]
          # Past a ` #` in double quotes, in a Nix string's, in single quotes
          # after a double-quoted `'`, past two lines ending in flags, and one
          # holding two spaces running.
          ++ map stale [
            "gone/i.js"
            "gone/k.js"
            "gone/l.js"
            "gone/j.js"
            "modules/a  dir/**"
          ]
          # A quoted pattern glued to a bare part, and to a quoted one.
          ++ map unreadable [
            "rg -g'${bang}modules/a/'b.js"
            "rg --glob '${bang}modules/a/'\"case.py\""
          ]
          # Past a ` #` in a double-quoted `$(`'s own quotes, before a `;#`,
          # and past an escaped space before a `#`.
          ++ map stale [
            "gone/z.js"
            "gone/w.js"
            "gone/y.js"
          ]
          # A `$(` given to rg named by its store path, quoted or not, or in a
          # string, a `${`, and a `$(` given to rg named by a variable or by
          # its store path's other spelling.
          ++ map unreadable [
            "\${lib.getExe pkgs.ripgrep} --glob \"${run}f)\""
            "/bin/rg --glob \"${run}f)\""
            "\"\${lib.getExe pkgs.ripgrep}\" --glob \"${run}f)\""
            "script = \"rg --glob \\\"${run}f)\\\"\";"
            "rg --glob ${brace}lib.escapeShellArg \"${bang}gone.js\"}"
            "\"$rg\" --glob \"${run}f)\""
            "\${lib.getExe' pkgs.ripgrep \"rg\"} --glob \"${run}f)\""
            "\${rg} --glob \"${run}f)\""
          ]
          # Past a comment after a Nix name's prime.
          ++ map stale [ "gone/v.js" ];
      };
      written = name: lines: pkgs.writeText name (lib.concatLines lines);
    in
    # A reading that found nothing would pass every tree.
    assert lib.assertMsg (here != [ ]) (
      "every-pattern-names-a-file read no ripgrep exclusion in tests/repository's .sh and "
      + ".nix files: a change to its reading has most likely stopped it finding them; "
      + "take this assert off only if none is meant to be left."
    );
    # A .sh file's `${`, the shell's own variable, is passed over.
    assert lib.assertMsg (leftOut "t.sh" "rg --glob \"${brace}dir}/x\"" == [ ]) (
      "every-pattern-names-a-file called a .sh file's rg --glob \"\${dir}/x\" unreadable: "
      + "the shell fills in a .sh file's own variables, which the reading passes over."
    );
    # A .sh file's `$(` still can't be read.
    assert lib.assertMsg
      (leftOut "t.sh" "rg --glob \"${run}f)\"" == [ "t.sh\t\trg --glob \"${run}f)\"" ])
      (
        "every-pattern-names-a-file passed a .sh file's rg --glob \"${run}f)\": "
        + "the reading should call a glob a command gives unreadable in any file."
      );
    pkgs.runCommandLocal "every-pattern-names-a-file" { } ''
      ${check} ${../../tools/merge-net/must-run.txt} ${../../ruff.toml} \
        ${written "unread" formatterExcludes} ${written "paths" paths} \
        ${written "checks" (lib.attrNames self.checks.x86_64-linux)} \
        ${written "left-out" here}
      if ${check} /dev/null /dev/null /dev/null ${written "made-up" madeUp.paths} /dev/null \
        ${written "made-up-left-out" (leftOut "t.nix" madeUp.script)} 2>said; then
        echo "It passed a made-up tree's patterns that name nothing." >&2; exit 1
      fi
      diff ${written "said" madeUp.said} said
      touch $out
    '';
}
