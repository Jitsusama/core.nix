# The checks of zathura, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  shellFolderLint,
  ...
}:
onLinux {
  # What zathura's provider says of what it has open, in Qt's own engine
  # (../../tests/zathura-program.qml, over Zathura.js): open-now's lines
  # read back, the page beside a document only one zathura has open, and
  # its drawer's facts saying each one's page and how far through it is.
  the-launcher-says-the-page-zathura-is-on =
    let
      qt = pkgs.qt6.qtdeclarative;
    in
    pkgs.runCommandLocal "the-launcher-says-the-page-zathura-is-on" { } ''
      cp ${../../tests/zathura-program.qml} zathura-program.qml
      cp ${./Zathura.js} Zathura.js
      export HOME=$TMPDIR LANG=C.UTF-8 QT_FORCE_STDERR_LOGGING=1 QT_QPA_PLATFORM=offscreen
      export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
      ${shellFolderLint}
      QML_IMPORT_PATH=${qt}/lib/qt-6/qml timeout 60 ${qt}/bin/qml zathura-program.qml
      touch $out
    '';

  # clippy finds nothing in ]b and [b's helper, its tests included.
  zathura-next-document-passes-clippy = passesClippy "zathura-next-document-passes-clippy" (
    pkgs.callPackage ./next-document/package.nix { }
  );

  # clippy finds nothing in what carries the launcher's command line to
  # zathura, its tests included.
  zathura-command-passes-clippy = passesClippy "zathura-command-passes-clippy" (
    pkgs.callPackage ./command/package.nix { }
  );

  # Escape closes nothing in zathura, its contents included: zathura's own
  # Escape, Ctrl+[ and Ctrl+C there are unmapped, and ZZ closes them as a
  # window, beside c, which opened them.
  zathura-escape-closes-no-contents =
    let
      zathura = (account graphical).programs.zathura;
      lines = lib.splitString "\n" zathura.extraConfig;
      unmapped = key: lib.elem "unmap [index] ${key}" lines;
    in
    holds "zathura-escape-closes-no-contents"
      "zathura's contents close on Escape, Ctrl+[ or Ctrl+C, or not on ZZ: unmap each in [index] and map [index] ZZ to toggle_index."
      (
        lib.all unmapped [
          "<Esc>"
          "<C-[>"
          "<C-c>"
        ]
        && (zathura.mappings."[index] ZZ" or "") == "toggle_index"
      );

  # zathura opens a page with the account's settings. It names a setting
  # or a colour it can't take and goes on without it, and stays open until
  # it's closed, so it's given a few seconds to say so. Its bar is in the
  # monospaced face, since it's a command line and page numbers. Documents
  # open in zathura.
  zathura-accepts-its-configuration =
    let
      home = account graphical;
      opens = graphical.config.xdg.mime.defaultApplications."application/pdf" or "";
      page = import ../../tests/lib/page.nix { inherit lib pkgs; };
      mono = home.jitsusama.theme.font.mono.family;
    in
    assert opens == "org.pwmt.zathura-pdf-mupdf.desktop";
    pkgs.runCommandLocal "zathura-accepts-its-configuration"
      {
        nativeBuildInputs = [
          home.programs.zathura.package
          pkgs.cage
        ];
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_DATA_HOME=$PWD/data
        export XDG_RUNTIME_DIR=$PWD/run GDK_BACKEND=wayland
        export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
        export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
        mkdir -p config/zathura data run
        chmod 700 run
        cp ${home.xdg.configFile."zathura/zathurarc".source} config/zathura/zathurarc
        grep -qE '^set font[[:space:]]+"${mono} ' config/zathura/zathurarc \
          || { echo "zathura's bar isn't in ${mono}:"; grep font config/zathura/zathurarc; exit 1; }
        # cage ends when zathura does, so only a zathura still open at the
        # end of the wait opened.
        status=0
        timeout 5 cage -- zathura ${page} > said 2>&1 || status=$?
        if [ $status -ne 124 ]; then cat said; exit 1; fi
        if grep -E '^(warning|error):' said; then exit 1; fi
        touch $out
      '';

  # Every key the launcher lists for zathura goes where it says, the page
  # each lands on read back from zathura itself over its bus and each
  # scroll from its screen, the document ]b and [b open from what it
  # says it has open, and : asking the launcher for its command line over
  # the window, whose commands zathura-command carries back over the bus,
  # a page and zathura's own alike, in a compositor of its own
  # (keys-test.sh). The
  # account's settings are read as they are, but for the word zathura
  # draws on a page not drawn yet, which would tell a page being drawn
  # from one that's done.
  zathura-keys-go-where-they-say =
    let
      home = account graphical;
      pages = import ../../tests/lib/page.nix {
        inherit lib pkgs;
        pages = 6;
        words = [
          ""
          ""
          ""
          "find"
          ""
          "find"
        ];
      };
      # Two documents beside it in its folder, for ]b and [b to go between.
      page = import ../../tests/lib/page.nix { inherit lib pkgs; };
      # The launcher, which only has to say it was asked.
      launcher = pkgs.writeShellScriptBin "launcher-say" ''echo "$*" > asked'';
    in
    pkgs.runCommandLocal "zathura-keys-go-where-they-say"
      {
        nativeBuildInputs = [
          home.programs.zathura.package
          launcher
          (pkgs.callPackage ./command/package.nix { })
          pkgs.cage
          pkgs.dbus
          pkgs.glib
          pkgs.grim
          pkgs.wtype
        ];
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_DATA_HOME=$PWD/data
        export XDG_RUNTIME_DIR=$PWD/run GDK_BACKEND=wayland
        export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
        export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
        mkdir -p config/zathura data run documents
        chmod 700 run
        cp ${pages} documents/b.pdf
        cp ${page} documents/a.pdf
        cp ${page} documents/c.pdf
        echo 'Not a document.' > documents/notes.txt
        cp ${home.xdg.configFile."zathura/zathurarc".source} config/zathura/zathurarc
        chmod u+w config/zathura/zathurarc
        echo 'set render-loading false' >> config/zathura/zathurarc
        dbus-run-session --config-file=${pkgs.dbus}/share/dbus-1/session.conf -- \
          cage -- $BASH ${./keys-test.sh} $PWD/documents/b.pdf > ran 2>&1 || true
        if [ ! -e passed ]; then cat ran; exit 1; fi
        touch $out
      '';

  # zathura says the document it has open, read over its bus by the command
  # its Recent in the launcher is led by, as its process and the whole
  # path of a document given by name in the folder it was started in, then
  # the page it's on, counted from one, and how many it has, and nothing
  # once ZZ closes it (../../tests/lib/says-what-it-has-open.sh), in a
  # compositor of its own.
  zathura-says-the-document-it-has-open =
    let
      home = account graphical;
      pages = import ../../tests/lib/page.nix {
        inherit lib pkgs;
        pages = 3;
      };
      source = lib.findFirst (
        each: each.app or "" == "org.pwmt.zathura"
      ) null home.jitsusama.quickshell.launcher.recents;
    in
    pkgs.runCommandLocal "zathura-says-the-document-it-has-open"
      {
        nativeBuildInputs = [
          home.programs.zathura.package
          pkgs.cage
          pkgs.dbus
          pkgs.wtype
        ];
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_DATA_HOME=$PWD/data
        export XDG_RUNTIME_DIR=$PWD/run GDK_BACKEND=wayland
        export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
        export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
        mkdir -p config/zathura data run
        chmod 700 run
        cp ${home.xdg.configFile."zathura/zathurarc".source} config/zathura/zathurarc
        cp ${pages} document.pdf
        dbus-run-session --config-file=${pkgs.dbus}/share/dbus-1/session.conf -- \
          cage -- $BASH ${../../tests/lib/says-what-it-has-open.sh} --closed-by ZZ \
          --then "$(printf '\t\t1\t3')" document.pdf \
          zathura -- ${lib.escapeShellArgs source.opened} > ran 2>&1 || true
        if [ ! -e passed ]; then cat ran; exit 1; fi
        touch $out
      '';

  # zathura says each file it's started with to the desktop's list of files
  # opened lately, through recent-files where the desktop has it, stood in
  # here, handing on all it was given, and then runs as it would.
  zathura-says-the-files-it-opens =
    let
      home = account graphical;
      recent-files = pkgs.writeShellScriptBin "recent-files" ''echo "$*" >> "$SAID"'';
    in
    pkgs.runCommandLocal "zathura-says-the-files-it-opens"
      {
        nativeBuildInputs = [
          home.programs.zathura.package
          recent-files
        ];
      }
      ''
        export HOME=$PWD SAID=$PWD/said
        zathura --version one.pdf > version 2>&1 || { cat version; exit 1; }
        grep -q -i zathura version || { echo "zathura didn't run: $(cat version)"; exit 1; }
        told=$(if [ -e said ]; then cat said; else echo 'nothing said'; fi)
        [ "$told" = "add zathura -- --version one.pdf" ] || { echo "zathura said: $told"; exit 1; }
        touch $out
      '';
}
