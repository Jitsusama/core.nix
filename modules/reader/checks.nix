# The checks of the reader, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  graphical,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
let
  home = account graphical;
  reader = lib.findFirst (package: lib.getName package == "reader") null home.home.packages;
  fonts = pkgs.makeFontsConf { fontDirectories = graphical.config.fonts.packages; };

  # A compositor of nobody's, drawn in software, with the account's theme
  # file where the reader looks for it.
  screen = ''
    export HOME=$PWD XDG_CONFIG_HOME=$PWD/config XDG_RUNTIME_DIR=$PWD/run
    export XDG_STATE_HOME=$PWD/state XDG_CACHE_HOME=$PWD/cache FONTCONFIG_FILE=${fonts}
    export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
    export LIBGL_ALWAYS_SOFTWARE=1
    export __EGL_VENDOR_LIBRARY_DIRS=${pkgs.mesa}/share/glvnd/egl_vendor.d
    mkdir -p config/reader run state cache
    chmod 700 run
    cp ${home.xdg.configFile."reader/theme.json".source} config/reader/theme.json
  '';

  # The colour as ImageMagick names a pixel's.
  srgb =
    colour:
    let
      hex = lib.removePrefix "#" colour;
      channel = index: toString (lib.fromHexString (builtins.substring (index * 2) 2 hex));
    in
    "srgb(${channel 0},${channel 1},${channel 2})";
in
# The reader draws through Wayland and EGL, on the Linux desktop.
onLinux {
  # The reader's tests run as it builds: a table's columns from the words
  # in them, every line of a table and of records on the text's pitch, a
  # picture known by its first bytes, a place kept as a block and a way into
  # it and read back as it was, a save heard whether written in place or
  # moved over and another file's not, the file shown said for the launcher
  # and taken away on closing, the theme's roles read with Osaka Jade for
  # the rest, search found across a hyphen's break, an equation shown as
  # written when MiTeX is missing, counts and keys as Neovim takes them, and
  # the motion's curve.
  reader-sets-its-pages = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in the reader, its tests included.
  reader-passes-clippy = passesClippy "reader-passes-clippy" (pkgs.callPackage ./package.nix { });

  # The keys the launcher lists over the reader's window are the reader's
  # own: every chord its table takes, with what it does and its group, as
  # reader --keys says them, and no other.
  reader-says-the-keys-it-takes =
    let
      listed = builtins.toJSON (
        lib.sort (a: b: a.chord < b.chord) (
          map (key: {
            inherit (key) chord does group;
          }) home.jitsusama.reader.keys
        )
      );
    in
    pkgs.runCommandLocal "reader-says-the-keys-it-takes" { nativeBuildInputs = [ pkgs.jq ]; } ''
      ${lib.getExe reader} --keys | jq -S -c 'sort_by(.chord)' > took
      echo ${lib.escapeShellArg listed} | jq -S -c . > listed
      if ! diff took listed; then
        echo "The launcher's list of the reader's keys (home.nix) isn't the reader's own (<) any more."
        exit 1
      fi
      touch $out
    '';

  # The reader draws a long page in the account's theme and takes Neovim's
  # keys: G goes to the end, gg to the top, 5j five lines, Ctrl+F a screen,
  # Ctrl+O back, Ctrl+Equal and Ctrl+0 zoom in and back, / finds and n goes
  # on, Space ? lists the keys, Escape puts the list away, g x labels the
  # link at the page's end and its label follows it to the heading, and q
  # closes it
  # (tests/reader-takes-its-keys.sh). The page is on the theme's background,
  # its heading in the strong text. Markdown files open in the reader.
  reader-takes-its-keys =
    let
      opens = graphical.config.xdg.mime.defaultApplications."text/markdown" or "";
      inherit (home.jitsusama.theme) colours;
    in
    assert opens == "reader.desktop";
    pkgs.runCommandLocal "reader-takes-its-keys"
      {
        nativeBuildInputs = [
          reader
          pkgs.cage
          pkgs.gawk
          pkgs.grim
          pkgs.imagemagick
          pkgs.inotify-tools
          pkgs.wtype
        ];
      }
      ''
        ${screen}
        {
          echo '# Every Key Neovim Has'
          for n in $(seq 120); do
            echo
            echo "Paragraph $n says enough to take a line or two of the page, so that every key here has somewhere to go."
          done
          echo
          echo '[Back to the top](#every-key-neovim-has)'
        } > page.md
        timeout 300 cage -- $BASH ${./tests/reader-takes-its-keys.sh} page.md page.png \
          '${srgb colours.strong}' > ran 2>&1 || true
        if [ ! -e passed ]; then cat ran said 2> /dev/null; exit 1; fi

        at() { magick "$1" -format "%[pixel:p{$2}]" info:; }
        [ "$(at page.png 2,2)" = "${srgb colours.background}" ] \
          || { echo "the page is $(at page.png 2,2), not the theme's background"; exit 1; }
        mkdir $out
        cp page.png $out
      '';

  # The reader keeps the file it shows, read by the command its Recent in
  # the launcher is led by as its process and the whole path of a file given
  # by name in the folder it was started in, and nothing once q closes it
  # (../../tests/says-what-it-has-open.sh). What a killed reader left is
  # cleared as it starts.
  reader-says-the-file-it-shows =
    let
      kept = "${home.xdg.stateHome}/reader/showing";
      source = lib.findFirst (
        each: each.app or "" == "reader"
      ) null home.jitsusama.quickshell.launcher.recents;
      # The launcher's command, reading the folder this check keeps.
      opened = lib.concatMapStringsSep " " (
        part: if part == kept then ''"$XDG_STATE_HOME/reader/showing"'' else lib.escapeShellArg part
      ) source.opened;
    in
    assert lib.elem kept source.opened;
    pkgs.runCommandLocal "reader-says-the-file-it-shows"
      {
        nativeBuildInputs = [
          reader
          pkgs.cage
          pkgs.wtype
        ];
      }
      ''
        ${screen}
        mkdir -p state/reader/showing
        printf '# A Sheet\n\nTo read.\n' > sheet.md
        printf '999999\t/gone.md\n' > state/reader/showing/999999
        : > state/reader/showing/.999999
        timeout 300 cage -- $BASH ${../../tests/says-what-it-has-open.sh} sheet.md \
          reader -- ${opened} > ran 2>&1 || true
        if [ ! -e passed ]; then cat ran; exit 1; fi
        if [ -n "$(ls -A state/reader/showing)" ]; then
          echo "the reader left this: $(ls -A state/reader/showing)"
          exit 1
        fi
        touch $out
      '';

  # The reader says each file it's started with to the desktop's list of
  # files opened lately, through recent-files where the desktop has it,
  # stood in here, handing on all it was given, and then runs as it would.
  reader-says-the-files-it-opens =
    let
      recent-files = pkgs.writeShellScriptBin "recent-files" ''echo "$*" >> "$SAID"'';
    in
    pkgs.runCommandLocal "reader-says-the-files-it-opens"
      {
        nativeBuildInputs = [
          reader
          recent-files
        ];
      }
      ''
        export HOME=$PWD SAID=$PWD/said
        reader --keys > keys 2>&1 || { cat keys; exit 1; }
        grep -q 'Close the page' keys || { echo "the reader didn't run: $(cat keys)"; exit 1; }
        told=$(if [ -e said ]; then cat said; else echo 'nothing said'; fi)
        [ "$told" = "add reader --keys" ] || { echo "the reader said: $told"; exit 1; }
        touch $out
      '';
}
