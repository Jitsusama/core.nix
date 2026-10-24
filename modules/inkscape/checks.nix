# The checks of inkscape, which checks.nix gathers with every other folder's;
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
  # Inkscape starts a new drawing from the account's profile, on the theme's
  # desk: a template it can't find leaves its own grey one without a word.
  # Inkscape draws nothing without a screen, so GTK reads the stylesheet
  # instead, refusing what it can't parse, and every name it recolours has
  # to be one Inkscape's own stylesheets still use.
  inkscape-accepts-its-configuration =
    let
      home = account graphical;
      inherit (home.jitsusama.theme) roles;
      inkscape = lib.findFirst (
        package: lib.getName package == "inkscape-with-extensions"
      ) null home.home.packages;
      profile = "${home.xdg.configHome}/inkscape";
      files = home.home.file;
      python = pkgs.python3.withPackages (packages: [ packages.pygobject3 ]);
      typelibs = lib.makeSearchPath "lib/girepository-1.0" [
        pkgs.gtk3
        pkgs.gdk-pixbuf
        pkgs.pango.out
        pkgs.harfbuzz
        pkgs.at-spi2-core
        pkgs.gobject-introspection
      ];
      parses = pkgs.writeText "parses.py" ''
        import gi, sys
        gi.require_version("Gtk", "3.0")
        from gi.repository import Gtk
        Gtk.CssProvider().load_from_path(sys.argv[1])
      '';
    in
    pkgs.runCommandLocal "inkscape-accepts-its-configuration"
      {
        nativeBuildInputs = [
          inkscape
          python
        ];
      }
      ''
        export HOME=$PWD INKSCAPE_PROFILE_DIR=$PWD/profile
        mkdir -p profile/ui profile/templates
        cp ${files."${profile}/ui/user.css".source} profile/ui/user.css
        cp ${files."${profile}/templates/default.svg".source} profile/templates/default.svg
        inkscape --actions='file-new;export-filename:new.svg;export-do' > said 2>&1 || true
        if ! grep -q 'inkscape:deskcolor="${roles.sunken}"' new.svg; then cat said; exit 1; fi
        GI_TYPELIB_PATH=${typelibs} python3 ${parses} profile/ui/user.css
        sed -n -E 's/^ *([^ /*].*[^ ]) *[{,]$/\1/p' profile/ui/user.css > selectors
        while read -r selector; do
          if ! grep -q -F -- "$selector" ${inkscape}/share/inkscape/ui/*.css; then
            echo "Inkscape's stylesheets no longer use $selector"
            exit 1
          fi
        done < selectors
        touch $out
      '';
}
