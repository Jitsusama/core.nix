# The checks of kitty, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  onLinux,
  pkgs,
  shellFolderLint,
  ...
}:
let
  home = account graphical;
  inherit (home.xdg) configFile;

  # The account's kitty files, in a folder as kitty reads them.
  configuration = pkgs.linkFarm "kitty-configuration" {
    "kitty.conf" = configFile."kitty/kitty.conf".source;
    "theme.conf" = configFile."kitty/theme.conf".source;
    "keys.conf" = configFile."kitty/keys.conf".source;
  };

  # A fontconfig that finds only the monospaced face, with what's given
  # besides, such as the account's features.
  fonts =
    extra:
    pkgs.writeText "fonts.conf" ''
      <?xml version="1.0"?>
      <!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
      <fontconfig>
        <dir>${pkgs.nerd-fonts.monaspace}/share/fonts</dir>
        <cachedir>cache</cachedir>
        ${extra}
      </fontconfig>
    '';
  plain = fonts "";
  featured = fonts "<include>${configFile."fontconfig/conf.d/90-hm-features.conf".source}</include>";

  # shape FONTS STYLE TEXT...: each text's glyphs, a line each, as the
  # account's kitty shapes them in that style with that fontconfig.
  shape = ''
    shape() {
      FONTCONFIG_FILE=$1 kitty +launch ${./shape.py} ${configuration}/kitty.conf "''${@:2}"
    }
  '';
in
onLinux {
  # What Programs › kitty lists of kitty's own and what it opens is
  # Kitty.js's and KittyOpens.js's, run in Qt's own engine over kitty's
  # `ls` as open-now gives it, niri's windows, a ~/.ssh/config, a
  # ~/.ssh/known_hosts, zsh's history, the repositories under ~/src and
  # zoxide's list (tests/kitty-tabs.qml).
  kitty-lists-its-tabs =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "kitty-lists-its-tabs" { } ''
      cp ${../../tests/kitty-tabs.qml} kitty-tabs.qml
      cp ${./Kitty.js} Kitty.js
      cp ${./KittyOpens.js} KittyOpens.js
      cp ${../quickshell/launcher/Ago.js} Ago.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml kitty-tabs.qml
      touch $out
    '';

  # kitty reads back its files the same way. It warns about a setting it
  # doesn't know and refuses a value it can't parse; it doesn't check that a
  # mapped action exists.
  kitty-accepts-its-configuration =
    pkgs.runCommandLocal "kitty-accepts-its-configuration" { nativeBuildInputs = [ pkgs.kitty ]; }
      ''
        export HOME=$PWD
        kitty +runpy 'from kitty.config import load_config; import sys; load_config(sys.argv[-1])' \
          ${configuration}/kitty.conf > complaints 2>&1
        if [ -s complaints ]; then cat complaints; exit 1; fi
        touch $out
      '';

  # kitty draws the monospaced face with the features the account's
  # fontconfig gives it: shaped through kitty's own font code, an equals
  # sign, a comparison and an arrow change their glyphs beside the face
  # alone, and the pipe and the range, whose sets are left off, don't.
  kitty-draws-the-ligatures =
    pkgs.runCommandLocal "kitty-draws-the-ligatures" { nativeBuildInputs = [ pkgs.kitty ]; }
      ''
        export HOME=$PWD
        ${shape}
        for text in '!=' '>=' '->'; do
          if [ "$(shape ${plain} regular "$text")" = "$(shape ${featured} regular "$text")" ]; then
            echo "$text is drawn without its ligature"
            exit 1
          fi
        done
        for text in '|>' '1..=2'; do
          if [ "$(shape ${plain} regular "$text")" != "$(shape ${featured} regular "$text")" ]; then
            echo "$text is drawn as a ligature, whose set is meant to be off"
            exit 1
          fi
        done
        touch $out
      '';

  # kitty draws the robot pi's footer marks a budget with in its own cell,
  # leaving the space after it blank, though the face inks it wider than a
  # cell: without narrow_symbols holding it to one, the space is inked.
  kitty-draws-the-budget-mark-in-one-cell =
    pkgs.runCommandLocal "kitty-draws-the-budget-mark-in-one-cell"
      { nativeBuildInputs = [ pkgs.kitty ]; }
      ''
        export HOME=$PWD
        # U+F06A9 in UTF-8's bytes, since bash writes a \U escape only in a
        # locale the build doesn't have.
        cells=$(FONTCONFIG_FILE=${featured} kitty +launch ${./ink.py} \
          ${configuration}/kitty.conf $'\xf3\xb0\x9a\xa9 d')
        if [ "$cells" != "inked blank inked" ]; then
          echo "the budget mark, a space and a letter are drawn as: $cells"
          exit 1
        fi
        touch $out
      '';

  # kitty draws every letter of a word as the letter alone, in each of its
  # four styles: texture healing, which gives the m of "limit" the room of
  # the narrow i on either side, would draw it in a glyph wider than its
  # cell, which kitty shrinks to fit.
  kitty-draws-every-letter-whole =
    pkgs.runCommandLocal "kitty-draws-every-letter-whole" { nativeBuildInputs = [ pkgs.kitty ]; }
      ''
        export HOME=$PWD
        ${shape}
        for style in regular bold italic bold-italic; do
          for word in limit will; do
            letters=$(shape ${featured} $style $(echo $word | grep -o .) | paste -sd ' ')
            if [ "$(shape ${featured} $style $word)" != "$letters" ]; then
              echo "$word in $style is drawn with a letter other than its own"
              exit 1
            fi
          done
        done
        touch $out
      '';
}
