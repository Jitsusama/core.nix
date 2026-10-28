# The checks of gimp, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  lib,
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # GIMP reads the files an account gets as it would at start: its system
  # preferences, where only GIMP's own copy of gimprc says a key exists; the
  # theme's palette, by name, with every pigment; and the stylesheet, which
  # GTK, GIMP's own copy, refuses if it can't parse it. A colour GIMP's
  # theme stops naming would be redefined to no effect, so the names have to
  # be the theme's exactly.
  gimp-accepts-its-configuration =
    let
      home = account graphical;
      inherit (home.jitsusama.theme) title appearance palette;
      gimp = lib.findFirst (package: lib.getName package == "gimp") null home.home.packages;
      folder = "${home.xdg.configHome}/GIMP/3.2";
      files = home.home.file;
      reads = pkgs.writeText "reads.py" ''
        import gi
        gi.require_version("Gtk", "3.0")
        from gi.repository import Gimp, Gtk
        assert Gimp.gimprc_query("theme-color-scheme") == "${appearance}", "no colour scheme"
        assert Gimp.gimprc_query("show-help-button") == "no", "no system gimprc"
        palette = Gimp.Palette.get_by_name("${title}")
        assert palette is not None, "no ${title} palette"
        assert palette.get_color_count() == ${toString (lib.length (lib.attrNames palette))}, "pigments missing"
        Gtk.CssProvider().load_from_path("GIMP/3.2/gimp.css")
        open("read", "w").close()
      '';
    in
    pkgs.runCommandLocal "gimp-accepts-its-configuration" { nativeBuildInputs = [ gimp ]; } ''
      export HOME=$PWD XDG_CONFIG_HOME=$PWD
      mkdir -p GIMP/3.2/palettes
      cp ${files."${folder}/gimp.css".source} GIMP/3.2/gimp.css
      cp ${files."${folder}/palettes/${title}.gpl".source} "GIMP/3.2/palettes/${title}.gpl"
      gimp-console -i --no-fonts --batch-interpreter=python-fu-eval -b - --quit < ${reads} \
        > said 2>&1 || true
      if [ ! -e read ] || grep -i 'gimprc' said; then cat said; exit 1; fi
      grep -o '@define-color [a-z-]*' ${gimp}/share/gimp/3.0/themes/Default/gimp-${appearance}.css \
        | sort > theirs
      grep -o '@define-color [a-z-]*' GIMP/3.2/gimp.css | sort > ours
      diff theirs ours
      touch $out
    '';

  # GIMP reads shortcutsrc only as its window starts, so its window starts,
  # headless, and quits once it has: it must say it read the account's file,
  # and nothing in it may be an action GIMP lacks, a key it can't parse or a
  # key another action already holds. It doesn't press ?, since no key
  # reaches a headless GIMP.
  gimp-takes-its-shortcuts =
    let
      home = account graphical;
      gimp = lib.findFirst (package: lib.getName package == "gimp") null home.home.packages;
      folder = "${home.xdg.configHome}/GIMP/3.2";
      file = home.home.file."${folder}/shortcutsrc";
    in
    pkgs.runCommandLocal "gimp-takes-its-shortcuts"
      {
        nativeBuildInputs = [
          gimp
          pkgs.cage
          pkgs.dbus
        ];
      }
      ''
        export HOME=$PWD XDG_CONFIG_HOME=$PWD XDG_RUNTIME_DIR=$PWD/run
        mkdir -m 700 run
        mkdir -p GIMP/3.2
        cp ${file.source} GIMP/3.2/shortcutsrc
        [ "${toString file.force}" = 1 ] || {
          echo "shortcutsrc isn't forced, so GIMP's own copy would stop the switch."
          exit 1
        }
        export GDK_BACKEND=wayland GSETTINGS_BACKEND=memory
        export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
        export XDG_DATA_DIRS=${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}
        export FONTCONFIG_FILE=${pkgs.makeFontsConf { fontDirectories = [ ]; }}
        dbus-run-session --config-file=${pkgs.dbus}/share/dbus-1/session.conf -- \
          cage -- gimp --verbose --no-splash --no-fonts --no-data --quit > said 2>&1 || true
        if ! grep -q "Parsing '$PWD/GIMP/3.2/shortcutsrc'" said; then
          cat said
          echo "GIMP never read the account's shortcutsrc."
          exit 1
        fi
        if grep -E 'shortcutsrc|invalid accelerator|duplicate accelerator|not existing action|unknown symbol' said \
          | grep -v "^Parsing '"; then
          echo "GIMP refused part of its shortcutsrc."
          exit 1
        fi
        touch $out
      '';
}
