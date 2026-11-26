# The checks of the launcher's own machinery: its prompt, rows, keys, pacing and
# answers, and the launcher on niri, which the folder's checks.nix gathers with
# every other place's; its opening comment says why each place keeps its own.
# Long because that comment keeps every check of the launcher's own
# machinery here, the parts every place's rows pass through, rather than
# with any one place.
{
  account,
  copied,
  copiedWith,
  wrongWith,
  programStepWith,
  graphical,
  launcherOnNiri,
  lib,
  mathsModules,
  noFonts,
  pkgs,
  qs,
  self,
  shellFolderLint,
  singleton,
  timedQml,
  ...
}:
let
  # The clipboard's history as the account runs it, serving Copies' and
  # Emoji's rows, so a place the launcher opens on niri has what Joel's has.
  clipboard = [
    "CLIPBOARD=${lib.getExe (pkgs.callPackage ../clipboard/package.nix { })}"
    "CLIPBOARD_WORDS=${(import ../launcher/words.nix).file}"
    "CLIPBOARD_EMOJI=${import ../launcher/emoji.nix { inherit lib pkgs; } "none"}"
  ];
in
{
  # The launcher answers what's typed into it, and takes vi's keys, on the
  # account's whole shell on niri, in seconds rather than
  # a machine's minutes (modules/quickshell/tests/launcher-on-niri-test.py). Its
  # meanings come from the meaning daemon's stand-in, which loads no model
  # (tests/desktop/stand-ins/stand-in-meaning.py). WARP stands in as warp-cli answering
  # from the files a test writes, the real warp reading them
  # (tests/stand-ins/warp-cli-stand-in.sh), and warp holds its link on the
  # socket the account's systemd would, as WARP, the shaper's status what a test
  # writes in ~/shaper, and the clipboard's history serves Copies' rows and
  # Emoji's. The home network is the eero daemon's stand-in, holding the
  # contract's network and the rows the daemon serves of it
  # (tests/desktop/stand-ins/stand-in-eero.py), as EERO.
  the-launcher-answers-whats-typed-on-niri =
    let
      warpCli = pkgs.writeShellScript "warp-cli" ''
        exec ${lib.getExe pkgs.bash} ${../../../tests/stand-ins/warp-cli-stand-in.sh} "$@"
      '';
      warp = lib.getExe (pkgs.callPackage ../../warp/package.nix { });
    in
    launcherOnNiri {
      name = "the-launcher-answers-whats-typed-on-niri";
      tests = ../tests/launcher-on-niri-test.py;
      # Six parts side by side, of about 80 s of cases each by their
      # times on 13 November (--durations), the timed trials kept together
      # in the last, and the degraded Work Network's minute-long wait alone
      # in the fifth, so it lengthens none; and each judges its own run's
      # log last.
      parts = [
        "test_an_answer_asked_for_holds_its_row_from_the_keystroke"
        "test_the_shut_shell_sleeps_as_a_window_is_resized"
        "test_a_cheap_drawer_comes_in_its_keys_own_frame"
        "test_now_says_a_degraded_work_network_within_its_line_and_steps_to_its_row"
        "test_a_terminal_is_shown_warps_state_health_and_round_trips_drawer"
      ];
      every = [ "test_no_answer_raised_an_error_and_quickshell_warned_of_nothing" ];
      given = clipboard ++ [
        "MEANING=${
          pkgs.writers.writePython3 "stand-in-meaning" {
            doCheck = false;
          } (builtins.readFile ../../../tests/desktop/stand-ins/stand-in-meaning.py)
        }"
        "EERO=${
          pkgs.writers.writePython3 "stand-in-eero" {
            doCheck = false;
          } (builtins.readFile ../../../tests/desktop/stand-ins/stand-in-eero.py)
        }"
        "EERO_NETWORK=${../../eero/home-network.json}"
        "EERO_ROWS=${../../eero/tests/rows.json}"
        "WARP_CONTRACT=${../../warp/contract}"
        "WARP=${pkgs.writeShellScript "warp-serve" ''
          WARP_CLI=${warpCli} WARP_SHAPER="$HOME/shaper/status.json" exec ${warp} serve "$@"
        ''}"
        # A virtual pointer, to click a margin mark.
        "WLRCTL=${lib.getExe pkgs.wlrctl}"
      ];
      shell.launcher.providers.eero.enable = true;
      # On the Selection without pi, its pi-verbs a case's stand-in.
      shell.launcher.providers.selection.enable = true;
      shell.launcher.providers.warp = {
        enable = true;
        warpCli = toString warpCli;
      };
    };

  # An opening draws home as the closed launcher kept it, each row as it stood,
  # then fills it in where its rows stand with nothing flashing; and keys typed
  # as it opens all land, and each way into a copy left before a share is let
  # go as it's pressed (modules/quickshell/tests/opening-on-niri-test.py); and chords said by
  # launcher-say as the shell restarts open it once it listens, and open it at
  # once all through a handover (../shell-hands-over.sh). Its `lab` times each
  # chord's opening, the one way every launcher change quotes (docs/testing.md's
  # Timing the Launcher), built only when asked for by name, since no gate runs
  # it (modules/quickshell/tests/launcher-lab-on-niri.py), and its `keys-lab`
  # each key typed from home as Joel types them, with Qt's profiler at hand for
  # a run that asks (modules/quickshell/tests/launcher-keys-lab-on-niri.py).
  # They're passed through, which outlasts the description checks.nix adds.
  the-launcher-opens-on-what-it-kept-on-niri =
    let
      # What niri runs on a chord, said by the cases through a restart.
      given = clipboard ++ [
        "LAUNCHER_SAY=${lib.getExe (pkgs.callPackage ../launcher-say/package.nix { })}"
        "HANDS_OVER=${lib.getExe (import ../hands-over.nix { inherit pkgs; })}"
      ];
    in
    (launcherOnNiri {
      name = "the-launcher-opens-on-what-it-kept-on-niri";
      tests = ../tests/opening-on-niri-test.py;
      inherit given;
      # The portal's ask what to open a file with, as the account's shell
      # holds it (../../open-with/home.nix), asked by a case.
      shell.launcher.providers.openWith = {
        order = 5;
        source = ../../open-with/OpenWith.qml;
      };
      parts = [
        "test_a_first_letter_after_a_restart_is_drawn_on_the_rows_it_lets_go"
        "test_home_kept_with_a_word_typed_fills_in_where_it_stood"
        "test_the_closed_launcher_draws_nothing_as_it_lets_its_closing_go"
        "test_for_you_makes_only_the_rows_whose_source_changed"
      ];
    }).overrideAttrs
      (old: {
        passthru = old.passthru or { } // {
          lab = launcherOnNiri {
            name = "the-launcher-lab-on-niri";
            tests = ../tests/launcher-lab-on-niri.py;
            inherit given;
          };
          keys-lab = launcherOnNiri {
            name = "the-launcher-keys-lab-on-niri";
            tests = ../tests/launcher-keys-lab-on-niri.py;
            helpers = [ ../tests/agents_served.py ];
            # The profiler of the Qt the shell runs on, beside what the lab
            # is given, without which Emoji and Copies have no rows.
            # And the realm's node and the agents daemon, which serve
            # Agents' rows from the sessions LAB_AGENTS lays, as the
            # account's units do.
            given = given ++ [
              "QMLPROFILER=${lib.getExe' graphical.pkgs.qt6.qtdeclarative "qmlprofiler"}"
              "MYCELIUM=${lib.getExe self.inputs.mycelium.packages.x86_64-linux.default}"
              "AGENTS=${lib.getExe (pkgs.callPackage ../../agents/package.nix { })}"
              "WORDS=${(import ../launcher/words.nix).file}"
            ];
            # Joel's caret rather than the checks' steady one, since what
            # it draws while he types is part of what a key waits for
            # (docs/testing.md).
            shell.caret.fade = lib.mkForce (account graphical).jitsusama.quickshell.caret.fade;
          };
        };
      });

  # With reports off, an ask from outside (a link, a catalogue run or a
  # terminal's do) says the words it was given by kind and how many letters,
  # and the shell's own failure by its error's name, since a page or a source
  # wrote them (modules/quickshell/tests/asked-from-outside-on-niri-test.py).
  the-launcher-says-asks-from-outside-by-kind-on-niri = launcherOnNiri {
    name = "the-launcher-says-asks-from-outside-by-kind-on-niri";
    tests = ../tests/asked-from-outside-on-niri-test.py;
    shell.reports = lib.mkForce false;
  };

  # With reports off, a lagging key's line says an arrow whole and a step by
  # its arrow and its row's letters, as each of keyTimed's callers in
  # Launcher.qml says its key is, a key listed without an answer and the
  # answer late count its letters, and a program's lagging drawer says its
  # desktop entry's id where that's shaped as an app's, and program otherwise
  # (modules/quickshell/tests/lagging-keys-on-niri-test.py).
  the-launcher-says-a-lagging-keys-kind-on-niri = launcherOnNiri {
    name = "the-launcher-says-a-lagging-keys-kind-on-niri";
    tests = ../tests/lagging-keys-on-niri-test.py;
    shell.reports = lib.mkForce false;
  };

  # Each of Super+E's groups has its emoji centred on its name as niri shows
  # the launcher, at 1.5 and 2 (modules/quickshell/tests/emoji-marks-on-niri-test.py).
  # It reads the launcher's own rows, which kept each emoji's measure with no
  # ink, drawn through Qt's GPU path on Mesa's software GL as the laptop's
  # draws on its GPU: an-emoji-stands-on-its-names-capitals draws each mark
  # anew, alone, and stood each emoji on its name while Joel's screen drew
  # them high.
  an-emoji-stands-on-its-name-on-niri = launcherOnNiri {
    name = "an-emoji-stands-on-its-name-on-niri";
    tests = ../tests/emoji-marks-on-niri-test.py;
    given = clipboard ++ [
      "QT_QUICK_BACKEND=rhi"
      "QSG_RHI_BACKEND=opengl"
    ];
  };

  # The launcher and a window share the keys by one rule: the newest Joel
  # opened takes them, a notice arriving never does, and a click on a
  # window gives it them (modules/quickshell/tests/focus-on-niri-test.py).
  the-newest-surface-takes-the-keys-on-niri = launcherOnNiri {
    name = "the-newest-surface-takes-the-keys-on-niri";
    tests = ../tests/focus-on-niri-test.py;
    given = [ "WLRCTL=${lib.getExe pkgs.wlrctl}" ];
  };

  # The launcher finds what's meant: for each query in
  # modules/quickshell/tests/fuzzy.qml the right thing comes first, few rows are
  # left, or nothing answers, and a query keeps within its budget in at least
  # one of five processes, all in Qt's own JavaScript engine, the one the shell
  # runs the matcher in; and every case of fuzzy.json, which the daemons'
  # matcher is held to too, scores and marks as it says.
  the-launcher-finds-what-is-meant = pkgs.runCommandLocal "the-launcher-finds-what-is-meant" { } ''
    cp ${../tests/fuzzy.qml} fuzzy.qml
    ${copied [ "Fuzzy.js" ]}
    cp ${../../ready-rows/fuzzy.json} fuzzy.json
    export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
    export FONTCONFIG_FILE=${noFonts} QML_XHR_ALLOW_FILE_READ=1
    ${timedQml "fuzzy.qml"}
    touch $out
  '';

  # The launcher makes a text ready to match without growing the shell's memory
  # by more than a few kilobytes a text, kept as it keeps them
  # (modules/quickshell/tests/fuzzy-light.qml), in a fresh process of Qt's own
  # JavaScript engine reading its own memory.
  the-launcher-makes-texts-ready-lightly =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-makes-texts-ready-lightly" { } ''
      cp ${../tests/fuzzy-light.qml} fuzzy-light.qml
      cp ${../../../tests/lib/Memory.js} Memory.js
      ${copied [ "Fuzzy.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} QML_XHR_ALLOW_FILE_READ=1
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 120 ${qt}/bin/qml fuzzy-light.qml
      touch $out
    '';

  # A ready row's name, made ready to match only once it's read, keeps nothing
  # alive but the row and what makes it ready, as
  # modules/quickshell/tests/member-light.qml holds launcher/Member.js to in a
  # fresh process of Qt's own JavaScript engine reading its own memory.
  the-launcher-makes-rows-lightly =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-makes-rows-lightly" { } ''
      cp ${../tests/member-light.qml} member-light.qml
      cp ${../launcher/Member.js} Member.js
      cp ${../../../tests/lib/Memory.js} Memory.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts} QML_XHR_ALLOW_FILE_READ=1
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 120 ${qt}/bin/qml member-light.qml
      touch $out
    '';

  # The launcher lands where it did: a thing is one row, the same letters
  # keep their row, and one slip teaches nothing, as
  # modules/quickshell/tests/ranking.qml holds launcher/Rank.js to it in Qt's
  # own engine, with launcher/Member.js making what a search found,
  # launcher/Fading.js what picks still count and launcher/Parts.js a
  # collection's parts listed once each.
  the-launcher-lands-where-it-did =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-lands-where-it-did" { } ''
      cp ${../tests/ranking.qml} ranking.qml
      cp ${../launcher/Rank.js} Rank.js
      cp ${../launcher/Member.js} Member.js
      cp ${../launcher/Parts.js} Parts.js
      cp ${../launcher/Fading.js} Fading.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml ${qt}/bin/qml ranking.qml
      touch $out
    '';

  # A list the launcher shows holds still as what's in it changes: its rows keep
  # their order, new ones come last and the selection keeps its row, as
  # modules/quickshell/tests/steady.qml holds launcher/Steady.js to it in Qt's
  # own engine.
  the-launcher-holds-still-as-it-refreshes =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-holds-still-as-it-refreshes" { } ''
      cp ${../tests/steady.qml} steady.qml
      cp ${../launcher/Steady.js} Steady.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml steady.qml
      touch $out
    '';

  # An opening joins the launcher's rows again each time a daemon answers,
  # walking only the sources that changed since and indexing the list in one
  # walk, as modules/quickshell/tests/joined.qml holds launcher/Joined.js to it
  # in Qt's own engine.
  the-launcher-joins-only-what-changed =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-joins-only-what-changed" { } ''
      cp ${../tests/joined.qml} joined.qml
      cp ${../launcher/Joined.js} Joined.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml joined.qml
      touch $out
    '';

  # A step the launcher holds says its row as it is now, and one whose row
  # now offers nothing is left, as modules/quickshell/tests/held.qml holds
  # launcher/Held.js to it in Qt's own engine.
  the-launcher-remakes-what-it-holds =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-remakes-what-it-holds" { } ''
      cp ${../tests/held.qml} held.qml
      cp ${../launcher/Held.js} Held.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml held.qml
      touch $out
    '';

  # copied's walk and guard, by what it makes of tables given it, so nothing
  # is built: it copies every file a root reaches, walked here apart from it,
  # and nothing else; a row past the root in another order copies the same;
  # and one left empty or dropped, or one two imports past the root naming
  # an import more, is refused, copying nothing, as the guard names it (wrongWith),
  # with imports.nix and that file said. Each case takes its file from imports.nix, so an edit to
  # one file's imports moves a case rather than failing it. Today's rows are
  # all sorted, so only this sees the order case break. A program's own file,
  # placed in a folder of its own, is copied there and brings what copying the
  # file it imports does, guarded as copying it is, its name quoted; one
  # importing a file the shell's folder doesn't hold, a folder, a file above
  # the shell's folder or one written from the root is refused, each on its
  # own escaped line naming the file, the import as written and what to do
  # instead, and the step stops. So is an own file given by anything but its
  # plain place, alone or beside those (programStepWith). A program's step
  # with no roots copies what its file reaches, and nothing else.
  a-check-copies-what-each-file-imports =
    let
      table = import ./imports.nix;
      root = "launcher/ReadyRows.qml";
      rowOf = file: table.${file} or [ ];
      entries = map (key: {
        inherit key;
      });
      reach =
        file:
        map (entry: entry.key) (
          lib.genericClosure {
            startSet = entries [ file ];
            operator = entry: entries (rowOf entry.key);
          }
        );
      reached = reach root;
      # A file past the root, since the walk gives the root first and the
      # cases must reach a row through another's, as a program's file does.
      several = lib.filter (file: file != root && lib.length (rowOf file) > 1) reached;
      picked = lib.head several;
      withTurned = table // {
        ${picked} = lib.reverseList (rowOf picked);
      };
      withEmptied = table // {
        ${picked} = [ ];
      };
      # With no row at all, as a file that had none gains its first import.
      withDropped = removeAttrs table [ picked ];
      # A file past the root's own row, so a guard reading only the root's
      # imports can't pass it, given a row naming an import it doesn't hold.
      deep = lib.findFirst (file: file != root && !(lib.elem file (rowOf root))) null reached;
      padding = lib.findFirst (file: file != deep && !(lib.elem file (rowOf deep))) null reached;
      withPadded = table // {
        ${deep} = rowOf deep ++ [ padding ];
      };
      real = copiedWith table [ root ];
      lines = lib.splitString "\n";
      copyLines = script: lib.filter (lib.hasPrefix "cp ") (lines script);
      at = pred: script: lib.lists.findFirstIndex pred null (lines script);
      making =
        folder: line:
        lib.hasPrefix "mkdir -p " line && lib.elem (lib.escapeShellArg folder) (lib.splitString " " line);
      uncopiedBy = script: lib.filter (file: !(lib.any (lib.hasSuffix " ${file}") (copyLines script)));
      uncopied = uncopiedBy real reached;
      leftOut = lib.concatStringsSep ", " uncopied;
      # Where each cp line copies to, so a file copied that nothing reaches,
      # or one copied twice, is seen as well as one left out: what's copied
      # past those wanted, or null.
      destination = line: lib.last (lib.splitString " " line);
      beyondOf =
        script: wanted:
        let
          copiedTo = map destination (copyLines script);
          beyond = lib.subtractLists wanted copiedTo;
        in
        if beyond != [ ] then
          lib.concatStringsSep ", " beyond
        else if lib.length copiedTo != lib.length (lib.unique copiedTo) then
          "a file twice"
        else
          null;
      realBeyond = beyondOf real reached;
      # A program's own file in a folder nothing of the shell's is in, so only
      # the step makes it, importing one as a file there names it.
      ownFolder = "planted";
      own = "${ownFolder}/Planted.js";
      ownImporting = written: {
        ${own} = builtins.toFile "Planted.js" ".import \"${written}\" as Planted\n";
      };
      planted = programStepWith table [ ] (ownImporting "../${picked}");
      unplanted = uncopiedBy planted ([ own ] ++ reach picked);
      plantedBeyond = beyondOf planted ([ (lib.escapeShellArg own) ] ++ reach picked);
      madeAt = at (making ownFolder) planted;
      ownCopiedAt = at (lib.hasSuffix " ${lib.escapeShellArg own}") planted;
      # A program's own file whose folder and name the shell must take as
      # they are.
      spacedFolder = "Planted spaced";
      spaced = "${spacedFolder}/Planted.js";
      spacedStep = lines (programStepWith table [ ] { ${spaced} = builtins.toFile "Planted.js" ""; });
      spacedUnquoted =
        !(lib.elem "mkdir -p ${lib.escapeShellArg spacedFolder}" spacedStep)
        || !(lib.any (lib.hasSuffix " ${lib.escapeShellArg spaced}") spacedStep);
      ownUnmade = madeAt == null || ownCopiedAt != null && madeAt > ownCopiedAt;
      ownHeld = builtins.pathExists (../. + "/${ownFolder}");
      stray = "Planted-nowhere.js";
      strayHeld = lib.any (file: builtins.pathExists (../. + "/${file}")) [
        stray
        quoted
      ];
      # Each import a program's file can't be given, as written in it, and
      # why the step must say it can't: a missing file, one whose name the
      # shell must take as it is written, a file above the shell's folder
      # that is there (the flake's own), and a folder, named or as ".."
      # alone, a file above the folder even when given as the program's own,
      # and one written from the root, which read beside its file names the
      # planted one. All are in one file, placed after the
      # planted one, so each must be said, of the file that imports it. The
      # program's own files given by anything but their plain place (above
      # the folder, from the root, through . or ..), imported or not, must
      # each be refused too, or they'd be copied elsewhere than said.
      quoted = "it's-$(gone).js";
      missing =
        file: "not in modules/quickshell; fix the import, or add it to the program's own files as ${file}";
      folder = "a folder; import a JavaScript file by name, or a type through qs with its file in programRoots (modules/quickshell/checks/helpers.nix)";
      above = "above modules/quickshell, where a check can't be given a file; move it in";
      unplain = "not its plain place in modules/quickshell; give it as that place, as launcher/Reader.js";
      ownAbove = "../Planted-above.js";
      outside = [
        ownAbove
        "../Planted-unimported.js"
        "/Planted-rooted.js"
        "./Planted-dotted.js"
        "${ownFolder}/../Planted-climbed.js"
      ];
      strays = {
        "../${stray}" = missing stray;
        "../${quoted}" = missing quoted;
        "../../../flake.nix" = above;
        "../${ownAbove}" = above;
        "../launcher" = folder;
        ".." = folder;
        "/${baseNameOf own}" =
          "written from the root, where a check can't be given a file; write it from this file";
      };
      strayFile = "${ownFolder}/Strays.js";
      # The strays alone, and beside the outside files, must each be said.
      strayedWith =
        given:
        lines (
          programStepWith table [ ] (
            ownImporting "../${picked}"
            // {
              ${strayFile} = builtins.toFile "Strays.js" (
                lib.concatMapStrings (written: ".import \"${written}\" as Stray\n") (lib.attrNames strays)
              );
            }
            // given
          )
        );
      strayedAlone = strayedWith { };
      strayed = strayedWith outsideGiven;
      bothSay = line: lib.elem line strayedAlone && lib.elem line strayed;
      outsideGiven = lib.genAttrs outside (_file: builtins.toFile "Planted-outside.js" "");
      # Given alone, with no import astray, they must still stop the step.
      outsideAlone = programStepWith table [ ] (ownImporting "../${picked}" // outsideGiven);
      outsideCopied = !(lib.elem "exit 1" (lines outsideAlone));
      sayingOf =
        written: "the program's file at ${strayFile} imports \"${written}\": ${strays.${written}}";
      strayUnrefused = lib.filter (
        written: !(bothSay "echo ${lib.escapeShellArg (sayingOf written)} >&2")
      ) (lib.attrNames strays);
      strayUnended = !(bothSay "exit 1");
      outsideLine =
        file: "echo ${lib.escapeShellArg "the program's own file given as \"${file}\": ${unplain}"} >&2";
      outsideUnrefused = lib.filter (
        file: !(lib.elem (outsideLine file) strayed && lib.elem (outsideLine file) (lines outsideAlone))
      ) outside;
      strayHeldSaid = "the shell's folder holds ${stray} or ${quoted}; give this check other names";
      ownHeldSaid = "the shell's folder holds ${ownFolder}; give this check another folder";
      plantedLeftOut = lib.concatStringsSep ", " unplanted;
      unplantedSaid = "a program's file importing ${picked} left out ${plantedLeftOut}";
      realSaidBeyond = "copying ${root} copied more than it reaches: ";
      plantedSaidBeyond = "a program's file importing ${picked}, with no roots, copied more: ";
      ownUnmadeSaid = "a program's file in ${ownFolder} was copied with no folder made first";
      unrefusedSaid = "a program's file importing these wasn't refused as each should be: ";
      # The guard by what it finds wrong, and a step it refuses by stopping
      # before it copies anything, saying imports.nix in full, for a
      # program's check in another folder, and the one file it names.
      stopped =
        script:
        let
          stopAt = at (line: line == "exit 1") script;
          copyAt = at (lib.hasPrefix "cp ") script;
        in
        stopAt != null && (copyAt == null || stopAt < copyAt);
      refusedNaming =
        file: script:
        stopped script
        && lib.hasInfix "modules/quickshell/checks/imports.nix doesn't say" script
        && lib.hasInfix "\"${file}\"" script;
      sameTurned = copiedWith withTurned [ root ] == real;
      emptiedRefused =
        wrongWith withEmptied [ root ] == [ picked ]
        && refusedNaming picked (copiedWith withEmptied [ root ]);
      droppedRefused =
        wrongWith withDropped [ root ] == [ picked ]
        && refusedNaming picked (copiedWith withDropped [ root ]);
      padded = copiedWith withPadded [ root ];
      paddedRefused = wrongWith withPadded [ root ] == [ deep ] && refusedNaming deep padded;
      plantedEmptied = programStepWith withEmptied [ ] (ownImporting "../${picked}");
      plantedEmptiedRefused =
        stopped plantedEmptied && lib.hasInfix (copiedWith withEmptied [ picked ]) plantedEmptied;
      plantedEmptiedSaid = "${picked}'s row, emptied, went unrefused from a program's file";
      failures =
        if wrongWith table [ root ] != [ ] then
          [
            "imports.nix's rows don't match these files: ${
              lib.concatStringsSep ", " (wrongWith table [ root ])
            }"
          ]
        else if several == [ ] then
          [ "no file ${root} reaches past itself imports two files; give this check another root" ]
        else if deep == null then
          [ "every file ${root} reaches is in its own row; give this check another root" ]
        else
          lib.optional (uncopied != [ ]) "copying ${root} left out ${leftOut}"
          ++ lib.optional (realBeyond != null) "${realSaidBeyond}${realBeyond}"
          ++ lib.optional (!sameTurned) "${picked}'s row, reversed, copied otherwise"
          ++ lib.optional (!emptiedRefused) "${picked}'s row, emptied, went unrefused"
          ++ lib.optional (!droppedRefused) "${picked}'s row, dropped, went unrefused"
          ++ lib.optional (!paddedRefused) "${deep}'s row, naming ${padding} too, went unrefused"
          ++ lib.optional (!plantedEmptiedRefused) plantedEmptiedSaid
          ++ lib.optional (unplanted != [ ]) unplantedSaid
          ++ lib.optional (plantedBeyond != null) "${plantedSaidBeyond}${plantedBeyond}"
          ++ lib.optional ownUnmade ownUnmadeSaid
          ++ lib.optional ownHeld ownHeldSaid
          ++ lib.optional strayHeld strayHeldSaid
          ++ lib.optional (strayUnrefused != [ ]) (unrefusedSaid + lib.concatStringsSep ", " strayUnrefused)
          ++ lib.optional strayUnended "a program's file importing those went on unstopped"
          ++ lib.optional (outsideUnrefused != [ ]) (
            "a program's own files given outside the shell's folder weren't refused: "
            + lib.concatStringsSep ", " outsideUnrefused
          )
          ++ lib.optional outsideCopied "a program's own files given outside the shell's folder alone were copied"
          ++ lib.optional spacedUnquoted "a program's own file at ${spaced} was made or copied unquoted";
    in
    pkgs.runCommandLocal "a-check-copies-what-each-file-imports" { } ''
      ${lib.concatMapStrings (failure: "echo ${lib.escapeShellArg failure} >&2\n") failures}
      ${lib.optionalString (failures != [ ]) "exit 1"}
      touch $out
    '';

  # Recent means one thing in every place it stands: the last nine things,
  # newest first, each once, never what's open, as
  # modules/quickshell/tests/recent.qml holds launcher/Recent.js to it in Qt's
  # own engine.
  recent-means-one-thing =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "recent-means-one-thing" { } ''
      cp ${../tests/recent.qml} recent.qml
      cp ${../launcher/Recent.js} Recent.js
      cp ${../launcher/Recents.js} Recents.js
      cp ${../launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml recent.qml
      touch $out
    '';

  # The open launcher holds its keys only once niri says no window has them, so
  # one reopened right after a closing, whose letting go reached niri late,
  # isn't closed as if a window took them (vm-desktop-looks, 3 of 6 runs,
  # 2026-11-11): modules/quickshell/tests/keys-held.qml replays each order of
  # events through launcher/KeysHeld.js in Qt's own engine.
  the-launcher-closes-only-for-keys-it-held =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-closes-only-for-keys-it-held" { } ''
      cp ${../tests/keys-held.qml} keys-held.qml
      cp ${../launcher/KeysHeld.js} KeysHeld.js
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml keys-held.qml
      touch $out
    '';

  # A key whose search awaits a daemon's answer is held a few milliseconds
  # for it, a key typed over it drops its search, and Return searches it at
  # once (Y2YBTS, 2026-11-11). Return inside those milliseconds can't be
  # timed on niri, so modules/quickshell/tests/awaited.qml holds
  # launcher/Awaited.js, which decides each and names what a key held past
  # its bound waited behind, in Qt's own engine.
  the-launcher-holds-a-key-for-its-answers =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-holds-a-key-for-its-answers" { } ''
      cp ${../tests/awaited.qml} awaited.qml
      ${copied [ "launcher/Awaited.js" ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml awaited.qml
      touch $out
    '';

  # The launcher's journal names what counted most, the most first and then by
  # name, and none that counted nothing, as modules/quickshell/tests/most.qml
  # holds launcher/Most.js to, in Qt's engine.
  the-launcher-says-what-counted-most =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-says-what-counted-most" { } ''
      cp ${../tests/most.qml} most.qml
      ${copied [ "launcher/Most.js" ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml most.qml
      touch $out
    '';

  # Answers a key's sources find after its search, heard in one turn, are taken
  # in once, and a key's own search drops what's gathered (VKCF3Z's owed red for
  # #4647). Three sources answering in one turn can't be made to on niri, so
  # modules/quickshell/tests/gathering.qml holds launcher/Gathering.qml, the
  # very component Launcher.qml gathers with, in Qt's own engine.
  late-answers-are-taken-in-once =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "late-answers-are-taken-in-once" { } ''
      cp ${../tests/gathering.qml} gathering.qml
      cp ${../launcher/Gathering.qml} Gathering.qml
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml gathering.qml
      touch $out
    '';

  # A place's row reads alike at home and inside its parent (VKCF3Z's
  # 2a429f67, on Joel's 2026-11-08 "i don't like how the title and values on
  # the home screen differ from every other sub-menu"):
  # modules/quickshell/tests/place-row.qml holds launcher/PlaceRow.js, which
  # makes it in both, in Qt's own engine, and home still unfilled a second after
  # an opening, each reading as being read.
  a-place-reads-alike-everywhere =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-place-reads-alike-everywhere" { } ''
      cp ${../tests/place-row.qml} place-row.qml
      cp ${../launcher/PlaceRow.js} PlaceRow.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml place-row.qml
      touch $out
    '';

  # A place tied to a part of the system says how it stands by the system
  # daemon's judgement of it (VKCF3Z's a1ef0c0a): nothing while it's good or the
  # daemon's silent, a muted word while it couldn't be read, and else the
  # daemon's own side in the warning's role or the alert's.
  # modules/quickshell/tests/system-part.qml holds launcher/SystemPart.js to it
  # in Qt's own engine.
  a-place-stands-by-its-part =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-place-stands-by-its-part" { } ''
      cp ${../tests/system-part.qml} system-part.qml
      cp ${../launcher/SystemPart.js} SystemPart.js
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml system-part.qml
      touch $out
    '';

  # The shell hears the system daemon's parts, by name, as the daemon tells
  # them, and holds none once the daemon is gone, so no old judgement reads as
  # current (modules/quickshell/tests/system-parts.qml), over a stand-in for the
  # daemon.
  the-shell-hears-the-systems-parts =
    # Kept in Nix because each line copies a file Nix gives it beside the
    # case, or sets what Quickshell runs it with.
    pkgs.runCommandLocal "the-shell-hears-the-systems-parts" { } ''
      mkdir -p runtime/daemons && chmod -R 700 runtime
      cp ${../../../tests/lib/Expect.js} Expect.js
      cp ${../../../tests/lib/Watch.js} Watch.js
      ${copied [
        "Daemon.qml"
        "SystemParts.qml"
      ]}
      cp ${singleton "Report.qml" ''
        function say(text) { }
      ''} Report.qml
      cp ${../tests/system-parts.qml} shell.qml
      export HOME=$TMPDIR XDG_RUNTIME_DIR=$TMPDIR/runtime LANG=C.UTF-8 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      touch $out
    '';

  # The rows drawn are told where they stand only when it changes, so a refresh
  # in the place they stay in redraws none of them:
  # modules/quickshell/tests/rows-place.qml holds launcher/RowsPlace.js to it in
  # Qt's own engine.
  the-rows-hear-only-a-new-place =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-rows-hear-only-a-new-place" { } ''
      cp ${../tests/rows-place.qml} rows-place.qml
      cp ${../launcher/RowsPlace.js} RowsPlace.js
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml rows-place.qml
      touch $out
    '';

  # Each kind of list keeps its one order with nothing typed (docs/design.md,
  # Lists): modules/quickshell/tests/order.qml holds launcher/Order.js's orders
  # to the rule in Qt's own engine, and
  # modules/quickshell/tests/every-list-keeps-its-kinds-order.py
  # walks each place that lists things: its sort its kind's order, no A to Z by
  # hand, and no order of Order.js's sorted by Qt's Array.sort, which shuffles
  # rows alike, as a stand-in sorting three ways it refuses shows.
  each-list-keeps-its-kinds-order =
    let
      qt = pkgs.qt6.qtdeclarative;
      unstable = pkgs.writeText "Unstable.js" ''
        const byName = Order.byName(one => one.name);
        const named = rows => rows.sort(byName);
        const live = rows => rows.sort(
            Order.live({ stands: one => one.state, name: one => one.name }));
        const wrapped = rows => rows.sort((a, b) => Order.byName(one => one.name)(a, b));
      '';
    in
    pkgs.runCommandLocal "each-list-keeps-its-kinds-order" { } ''
      lint="${lib.getExe pkgs.python3} ${../tests/every-list-keeps-its-kinds-order.py}"
      $lint ${../launcher} ${../../people}
      mkdir unstable
      cp ${unstable} unstable/Unstable.js
      if $lint unstable 2> said; then
        echo "a sort by Order.js through Array.sort passed" >&2
        exit 1
      fi
      for line in 2 3 5; do
        if ! grep -qF "Unstable.js:$line sorts by Order.js with Qt's Array.sort" said; then
          echo "Unstable.js:$line's sort by Order.js through Array.sort wasn't refused:" >&2
          cat said >&2
          exit 1
        fi
      done
      cp ${../tests/order.qml} order.qml
      cp ${../launcher/Order.js} Order.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml order.qml
      touch $out
    '';

  # A shut launcher counts the holders of its heap once a shell and weighs
  # what letting go of each of the largest would free, keeping what a busy
  # answer or an opening put off for the next closing, as
  # modules/quickshell/tests/weighing.qml holds launcher/Weighing.js to it
  # in Qt's own engine.
  the-launcher-weighs-what-it-owes =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-weighs-what-it-owes" { } ''
      cp ${../tests/weighing.qml} weighing.qml
      cp ${../launcher/Weighing.js} Weighing.js
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml weighing.qml
      touch $out
    '';

  # A row an act asked a daemon to change is drawn as it will be until its
  # source agrees, as modules/quickshell/tests/becoming.qml holds
  # launcher/Becoming.js to it in Qt's own engine.
  an-act-shows-before-its-daemon-answers =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "an-act-shows-before-its-daemon-answers" { } ''
      cp ${../tests/becoming.qml} becoming.qml
      cp ${../launcher/Becoming.js} Becoming.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml becoming.qml
      touch $out
    '';

  # An open launcher takes a burst of its sources' changes a pace apart, and
  # never two refreshes closer than that however the first was asked for, and
  # a source's rows are wanted only where they're shown or searched, and a
  # key's act is said shown by the first frame begun after its refresh, and
  # an opening weighs the foot's verbs only once its words or face change,
  # and a daemon's changes are said presented by place, and they and keys
  # with where their time went, and a closing makes again only the spots of
  # things whose stamp moved, and only the newest yank's frame starts its
  # flash, and an opening the collector stalled says where the heap stood
  # as it was asked, as modules/quickshell/tests/pacing.qml holds
  # launcher/ChangesPresented.js, launcher/Collector.js, launcher/Flash.js,
  # launcher/Pacing.js, launcher/Presented.js with its launcher/KeysSaid.js,
  # launcher/Spots.js, launcher/Thread.js and launcher/Wanted.js to it in
  # Qt's own engine.
  the-launcher-paces-its-refreshes =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    # Kept in Nix because each line copies a file Nix gives it beside the
    # cases, or sets what Qt's engine runs them with.
    pkgs.runCommandLocal "the-launcher-paces-its-refreshes" { } ''
      cp ${../tests/pacing.qml} pacing.qml
      cp ${../launcher/ChangesPresented.js} ChangesPresented.js
      cp ${../launcher/Collector.js} Collector.js
      cp ${../launcher/Flash.js} Flash.js
      cp ${../launcher/KeysSaid.js} KeysSaid.js
      cp ${../launcher/Makings.js} Makings.js
      cp ${../launcher/Order.js} Order.js
      cp ${../launcher/Pacing.js} Pacing.js
      cp ${../launcher/Presented.js} Presented.js
      cp ${../launcher/Spots.js} Spots.js
      cp ${../launcher/Thread.js} Thread.js
      cp ${../launcher/Wanted.js} Wanted.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml pacing.qml
      touch $out
    '';

  # A thing's drawer says each of its parts as a fact, leaving out its
  # acts and opening a holder of figures in place, as
  # modules/quickshell/tests/plate.qml holds launcher/Plate.js to it in
  # Qt's own engine. A screen's drawer, its settings, its light and what its
  # EDID says are the windows daemon's, tested there.
  a-things-drawer-says-its-parts =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    # Kept in Nix because each line copies, or names in a variable, a file
    # Nix gives it, and the cases it runs are a QML file of their own
    # (modules/quickshell/tests/plate.qml).
    pkgs.runCommandLocal "a-things-drawer-says-its-parts" { } ''
      cp ${../tests/plate.qml} plate.qml
      cp ${../launcher/Plate.js} Plate.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml plate.qml
      touch $out
    '';

  # Each opening of the launcher is kept as what it took: its keys by kind,
  # its pick and how long it took, and a miss only where letters were typed
  # and nothing picked, as modules/quickshell/tests/trials.qml holds
  # launcher/Trials.js and launcher/Stepped.js to it in Qt's own engine.
  the-launcher-keeps-its-trials =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-keeps-its-trials" { } ''
      cp ${../tests/trials.qml} trials.qml
      cp ${../launcher/Trials.js} Trials.js
      cp ${../launcher/Stepped.js} Stepped.js
      cp ${../launcher/Order.js} Order.js
      cp ${../launcher/RowsKept.js} RowsKept.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml trials.qml
      touch $out
    '';

  # Each close adds its own opening's line to the file's end and rewrites none
  # before it, so a close costs the same however many are kept: it turned all
  # 1,339 of Joel's into text, 39 ms a close at a load of 66 (2026-11-10). The
  # shell starting carries the old list into lines and lets the oldest go
  # (modules/quickshell/tests/trials-kept.qml, through Quickshell). With
  # reports off, a slow key's line says how many letters were typed and a
  # slow step's keeps its arrow, so neither names the row
  # (modules/quickshell/tests/trials-kept-heard.sh).
  the-launcher-keeps-its-trials-one-by-one =
    # procps gives the case pkill, to end the pipe as a full disk would.
    pkgs.runCommandLocal "the-launcher-keeps-its-trials-one-by-one"
      {
        nativeBuildInputs = [ pkgs.procps ];
        LANG = "C.UTF-8";
        QT_QPA_PLATFORM = "offscreen";
        FONTCONFIG_FILE = noFonts;
      }
      ''
        export HOME=$TMPDIR XDG_STATE_HOME=$PWD/state XDG_RUNTIME_DIR=$TMPDIR/runtime
        mkdir -m 700 launcher $XDG_RUNTIME_DIR
        ${copied [
          "Options.qml"
          "Trials.qml"
        ]}
        cp ${../tests/trials-kept.qml} shell.qml
        cp ${../../../tests/lib/Expect.js} Expect.js
        ${shellFolderLint}
        set -o pipefail
        timeout 60 ${qs} -p shell.qml 2>&1 | tee said
        ${pkgs.lib.getExe pkgs.bash} ${../tests/trials-kept-heard.sh} said
        touch $out
      '';

  # Each opening of the launcher is a trace: its steps, backs, acts and searches
  # spans under it, a void above a list's first row told from a list scrolled
  # down, and the whole sent in OTLP's JSON mapping, as
  # modules/quickshell/tests/traces.qml holds launcher/Traces.js to it in Qt's
  # own engine.
  the-launcher-traces-each-opening =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-traces-each-opening" { } ''
      cp ${../tests/traces.qml} traces.qml
      cp ${../launcher/Traces.js} Traces.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml traces.qml
      touch $out
    '';

  # A row the launcher listed before is kept wherever it moves, and only a row
  # new to the list is drawn afresh, as modules/quickshell/tests/rows-kept.qml
  # holds launcher/RowsKept.js to it in Qt's own engine.
  a-row-kept-is-never-drawn-again =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-row-kept-is-never-drawn-again" { } ''
      cp ${../tests/rows-kept.qml} rows-kept.qml
      cp ${../launcher/RowsKept.js} RowsKept.js
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml rows-kept.qml
      touch $out
    '';

  # The launcher answers what's asked: sums and number bases, worked out by
  # the shell's maths as the shell works them out, IPv4 and IPv6
  # addresses and networks with the kind of range each is in, subnet maths
  # (a network's parts, what holds an address, a prefix alone and a network
  # split), characters and their codes, named as Unicode names them, hardware
  # addresses, measures, the time in any zone tzdata names and the
  # machine's figures from its files, its parts' heat from their sensors,
  # the keyboards in the kernel's list of input devices, how each Bluetooth
  # link is doing from what bluetooth-links writes, what Sound makes of a
  # sound device's properties, the machine's connections from what ip, the
  # routing tables, resolv.conf and wifid say, and how WARP's link stands from
  # the record warp prints (modules/warp/warp.json), as
  # modules/quickshell/tests/answers/ lists them a place a file, and leaves
  # alone what isn't one, in Qt's own JavaScript engine.
  the-launcher-answers-what-is-asked =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-answers-what-is-asked" { } ''
      cp ${../tests/answers.qml} answers.qml
      cp -r ${../tests/answers} answers
      cp ${../launcher/Numbers.js} Numbers.js
      cp ${../launcher/Addresses.js} Addresses.js
      cp ${../launcher/Subnets.js} Subnets.js
      cp ${../launcher/Characters.js} Characters.js
      cp ${import ../launcher/names.nix { inherit lib pkgs; }} names.txt
      cp ${../launcher/Units.js} Units.js
      cp ${../launcher/Zones.js} Zones.js
      cp ${../launcher/Moment.js} Moment.js
      cp ${../launcher/NowDrawers.js} NowDrawers.js
      cp ${../launcher/Keeping.js} Keeping.js
      cp ${../launcher/Here.js} Here.js
      cp ${../launcher/Colours.js} Colours.js
      cp ${../launcher/Asking.js} Asking.js
      cp ${../launcher/Machine.js} Machine.js
      cp ${../launcher/Heat.js} Heat.js
      cp ${../launcher/Waits.js} Waits.js
      cp ${../launcher/Power.js} Power.js
      cp ${../launcher/Seen.js} Seen.js
      cp ${../launcher/Awake.js} Awake.js
      cp ${../launcher/Standing.js} Standing.js
      cp ${../launcher/Traffic.js} Traffic.js
      cp ${../launcher/Interfaces.js} Interfaces.js
      cp ${../launcher/Networkd.js} Networkd.js
      cp ${../launcher/Links.js} Links.js
      cp ${../launcher/Sounds.js} Sounds.js
      cp ${../launcher/Cameras.js} Cameras.js
      cp ${../launcher/Warp.js} Warp.js
      cp ${../launcher/WarpSeen.js} WarpSeen.js
      cp ${../launcher/WarpSwitch.js} WarpSwitch.js
      cp ${../launcher/WarpWaiting.js} WarpWaiting.js
      cp ${../launcher/WarpDrawer.js} WarpDrawer.js
      cp ${../launcher/HomeNetwork.js} HomeNetwork.js
      cp ${../launcher/Ago.js} Ago.js
      cp ${(import ../launcher/words.nix).file} words.json
      cp ${../../warp/warp.json} warp.json
      cp ${../launcher/Keyboards.js} Keyboards.js
      cp ${../launcher/KeyCounts.js} KeyCounts.js
      cp ${../launcher/Order.js} Order.js
      cp ${../launcher/TalkOn.js} TalkOn.js
      cp ${../launcher/SessionRow.js} SessionRow.js
      cp ${../tests/colour-cases.js} colour-cases.js
      cp ${import ../launcher/zones.nix { inherit lib pkgs; }} zones.jsonl
      export QML_XHR_ALLOW_FILE_READ=1
      # A zone with summer time, for the day of the year.
      export TZ=America/Toronto TZDIR=${pkgs.tzdata}/share/zoneinfo
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      # A script error never reaches Qt.exit, so without a limit the check
      # would wait forever rather than fail.
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml:${mathsModules} timeout 120 ${qt}/bin/qml answers.qml
      touch $out
    '';

  # The launcher's sums and measures, through the shell's maths, take a
  # typical key well inside its budget, each input typed a letter at a time
  # (modules/quickshell/tests/maths-speed.qml), which builds the maths'
  # module too.
  the-launcher-sums-within-a-key =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-sums-within-a-key" { } ''
      cp ${../tests/maths-speed.qml} maths-speed.qml
      cp ${../launcher/Numbers.js} Numbers.js
      cp ${../launcher/Units.js} Units.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml:${mathsModules} timeout 300 ${qt}/bin/qml maths-speed.qml
      touch $out
    '';

  # A long name slides at one reading pace between soft ends, as
  # modules/quickshell/tests/reading.qml says, in Qt's own engine.
  a-long-name-reads-at-one-pace =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-long-name-reads-at-one-pace" { } ''
      cp ${../tests/reading.qml} reading.qml
      ${copied [
        "Measures.js"
        "Reading.js"
      ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml reading.qml
      touch $out
    '';

  # Words that break out of the box they're laid in, the card, or onto another's
  # are named, as modules/quickshell/tests/contained.qml holds Contained.js, the
  # judge the launcher's `contained` asks on niri.
  no-words-leave-their-room =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "no-words-leave-their-room" { } ''
      cp ${../tests/contained.qml} contained.qml
      ${copied [ "Contained.js" ]}
      cp ${../../../tests/lib/Expect.js} Expect.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml contained.qml
      touch $out
    '';

  # A card says how it stands in one status line, the launcher's and held
  # notices' alike, as modules/quickshell/tests/status-line.qml holds
  # StatusLine.js to: which hints give way as room runs short, the keys list's
  # and Return's staying.
  a-card-says-how-it-stands-in-one-row =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "a-card-says-how-it-stands-in-one-row" { } ''
      cp ${../tests/status-line.qml} status-line.qml
      ${copied [ "StatusLine.js" ]}
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml status-line.qml
      touch $out
    '';

  # A program's act on a copy or on the repositories under ~/src shows only
  # while it can act, and opens what it says: what decides it, in Qt's own
  # engine (modules/quickshell/tests/quick-acts.qml, over Copied.js and
  # Neovim's NeovimOpening.js and Neovim.js), then each program's
  # provider with the shell's one watcher of the clipboard, through
  # Quickshell itself, over repositories made here and served once for both
  # programs by the real programs daemon (launcher/Repositories.qml),
  # with niri and the clipboard's history stood in
  # (modules/quickshell/tests/program-acts.qml). Chrome's downloads are read
  # from Chrome's ChromeDownloads.js in Qt's engine and through its provider
  # from the real programs daemon, running a stand-in for chrome-recent-sites,
  # once Chrome is stepped into: file rows in its Recent, each drawer saying
  # where it is then its kind and previewing a picture, and Open the Latest
  # Download only while there's one, waiting in its place until the first
  # reading answers. Neovim's files open now come through the same daemon from a
  # stand-in open-now, whose reading of a real Neovim is open-with's own tests',
  # each file's drawer saying where it is and its buffer.
  the-launcher-acts-only-on-what-it-can =
    let
      qt = pkgs.qt6.qtdeclarative;
      # A kitty used last, whose remote control Neovim opens through.
      niri = singleton "Niri.qml" ''
        property var windows: ({
          "4": { id: 4, app_id: "kitty", pid: 40, focus_timestamp: { secs: 10, nanos: 0 } },
          "5": { id: 5, app_id: "kitty", pid: 41, focus_timestamp: { secs: 20, nanos: 0 } }
        })
        property var spawned: []
        property var acted: []
        function spawn(command) { spawned = command; }
        function act(action) { acted = acted.concat([action]); }
      '';
      # What open-now says two Neovims in one kitty hold, as
      # modules/quickshell/tests/neovim-open-now.sh says.
      openNow = pkgs.writeShellScript "open-now" (builtins.readFile ../tests/neovim-open-now.sh);
      # file-preview stood in, saying which process answered.
      previewer = pkgs.writeShellScript "file-preview" (builtins.readFile ../tests/file-preview.sh);
      programs = pkgs.callPackage ../../programs/package.nix { };
      # Chrome's downloads, as chrome-recent-sites prints them: one from a
      # page, finished at a time of its own, one Chrome kept no time or
      # page for, and a picture; the programs daemon runs it as Chrome's
      # module has it run its own.
      downloads = pkgs.writeShellScript "downloads-list" ''
        printf '%s\t%s\t%s\t%s\n' "$HOME/Downloads/report.pdf" 2048 1800000100 \
          'https://docs.example.com/q3?x=1' "$HOME/Downloads/undated.zip" 10 "" "" \
          "$HOME/Downloads/tide-chart.png" 84300 1800000050 'https://tide.example.org/'
      '';
      sources = pkgs.writeText "sources.json" (
        builtins.toJSON [
          {
            app = "google-chrome";
            name = "downloads";
            asked = true;
            list = [ "${downloads}" ];
          }
          # What each Neovim has open, which the programs daemon reads as
          # Neovim's module has it read.
          {
            app = "nvim";
            name = "open";
            asked = true;
            list = [
              "${openNow}"
              "neovim"
            ];
          }
        ]
      );
      # Three repositories a folder deep, alpha's git written last, alpha's
      # origin and gamma's upstream on GitHub and beta's on GitLab, and one
      # four folders deep, past where they're looked for; real ones, since
      # git lists a repository's files.
      git = lib.getExe pkgs.git;
      repositories = ''
        repository() {
          ${git} init --quiet "$HOME/src/$1"
          ${git} -C "$HOME/src/$1" remote add "$2" "$3"
          touch -d "@$4" "$HOME/src/$1/.git"
        }
        mkdir -p $HOME/src/alpha/docs $HOME/src/beta
        printf 'fn main() {}\n' > $HOME/src/alpha/main.rs
        printf 'KEY=1\n' > $HOME/src/alpha/.env
        printf '# Notes\n' > $HOME/src/alpha/docs/notes.md
        printf 'ignored.txt\n' > $HOME/src/alpha/.gitignore
        printf 'not listed\n' > $HOME/src/alpha/ignored.txt
        touch -d @1700000400 $HOME/src/alpha/main.rs
        touch -d @1700000300 $HOME/src/alpha/.env
        touch -d @1700000200 $HOME/src/alpha/docs/notes.md
        touch -d @1700000100 $HOME/src/alpha/.gitignore
        printf '# Beta\n' > $HOME/src/beta/README.md
        repository alpha origin git@github.com:Jitsusama/alpha.git 1700000900
        repository beta origin https://gitlab.com/someone/beta 1700000800
        repository gamma upstream https://github.com/NixOS/gamma.git 1700000700
        repository far/away/too/deep origin git@github.com:someone/deep.git 1700000950
        repository .hidden/omega origin git@github.com:someone/omega.git 1700000960
        # A document outside them, for zathura's act.
        printf '%%PDF-1.4\n' > $HOME/report.pdf
      '';
    in
    pkgs.runCommandLocal "the-launcher-acts-only-on-what-it-can" { } ''
      mkdir cases shell
      cd $TMPDIR/cases
      cp ${../tests/quick-acts.qml} quick-acts.qml
      ${copied [ "Copied.js" ]}
      cp ${../../neovim/NeovimOpening.js} NeovimOpening.js
      cp ${../../chrome/ChromeDownloads.js} ChromeDownloads.js
      cp ${../../neovim/Neovim.js} Neovim.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${noFonts}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml quick-acts.qml
      cd $TMPDIR/shell
      mkdir -p launcher
      cp ${(account graphical).xdg.configFile."quickshell/Theme.qml".source} Theme.qml
      ${copied [
        "Copied.qml"
        "Daemon.qml"
        "Files.qml"
        "Words.qml"
        "launcher/Ago.js"
        "launcher/Captured.js"
        "launcher/Listed.qml"
        "launcher/Previewer.qml"
        "launcher/Provider.qml"
        "launcher/Repositories.qml"
        "launcher/words.json"
      ]}
      cp ${niri} Niri.qml
      cp ${../../neovim/NeovimProgram.qml} launcher/NeovimProgram.qml
      cp ${../../neovim/NeovimOpening.js} launcher/NeovimOpening.js
      cp ${../../neovim/Neovim.js} launcher/Neovim.js
      cp ${../../chrome/ChromeProgram.qml} launcher/ChromeProgram.qml
      cp ${../../chrome/ChromeDownloads.js} launcher/ChromeDownloads.js
      cp ${../../reader/ReaderProgram.qml} launcher/ReaderProgram.qml
      cp ${../../zathura/ZathuraProgram.qml} launcher/ZathuraProgram.qml
      cp ${../../zathura/Zathura.js} launcher/Zathura.js
      cp ${../../reader/Reader.js} launcher/Reader.js
      cp ${../../gimp/CopiedPicture.qml} launcher/CopiedPicture.qml
      cp ${../tests/program-acts.qml} shell.qml
      cp ${../../../tests/lib/Frames.js} Frames.js
      export XDG_RUNTIME_DIR=$TMPDIR/runtime
      export PATH=${
        lib.makeBinPath [
          pkgs.fd
          pkgs.coreutils
          pkgs.gnugrep
        ]
      }:$PATH
      export OPEN_NOW=${openNow}
      mkdir -m 700 $XDG_RUNTIME_DIR
      ${repositories}
      mkdir $XDG_RUNTIME_DIR/daemons
      XDG_DATA_HOME=$TMPDIR/data XDG_DATA_DIRS=$TMPDIR/data PROGRAMS_GIT=${git} \
        PROGRAMS_PREVIEW=${previewer} \
        ${programs}/bin/programs $XDG_RUNTIME_DIR/daemons/programs.sock ${sources} &
      daemon=$!
      ${shellFolderLint}
      timeout 60 ${qs} -p shell.qml
      kill $daemon
      touch $out
    '';
}
