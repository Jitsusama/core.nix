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
}
