# The checks of kitty, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  graphical,
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # kitty reads back its files the same way. It warns about a setting it
  # doesn't know and refuses a value it can't parse; it doesn't check that a
  # mapped action exists.
  kitty-accepts-its-configuration =
    let
      files = (account graphical).xdg.configFile;
    in
    pkgs.runCommandLocal "kitty-accepts-its-configuration" { nativeBuildInputs = [ pkgs.kitty ]; } ''
      export HOME=$PWD
      mkdir kitty
      cp ${files."kitty/kitty.conf".source} kitty/kitty.conf
      cp ${files."kitty/theme.conf".source} kitty/theme.conf
      cp ${files."kitty/keys.conf".source} kitty/keys.conf
      kitty +runpy 'from kitty.config import load_config; import sys; load_config(sys.argv[-1])' \
        kitty/kitty.conf > complaints 2>&1
      if [ -s complaints ]; then cat complaints; exit 1; fi
      touch $out
    '';

  # kitty draws the monospaced face with the features the account's
  # fontconfig gives it: shaped through kitty's own font code, an equals
  # sign, a comparison and an arrow change their glyphs beside the face
  # alone, and the pipe and the range, whose sets are left off, don't.
  kitty-draws-the-ligatures =
    let
      home = account graphical;
      inherit (home.jitsusama.theme.font) mono;
      features = home.xdg.configFile."fontconfig/conf.d/90-hm-features.conf".source;
      conf =
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
      shape = pkgs.writeText "shape.py" ''
        import sys
        from kitty.fonts.render import shape_string
        text = sys.argv[-1]
        print(" ".join(str(glyphs) for *_, glyphs in shape_string(text, family="${mono.family}")))
      '';
    in
    pkgs.runCommandLocal "kitty-draws-the-ligatures" { nativeBuildInputs = [ pkgs.kitty ]; } ''
      export HOME=$PWD
      shaped() {
        FONTCONFIG_FILE=$1 kitty +runpy "exec(open('${shape}').read())" "$2"
      }
      for text in '!=' '>=' '->'; do
        if [ "$(shaped ${conf ""} "$text")" = "$(shaped ${conf "<include>${features}</include>"} "$text")" ]; then
          echo "$text is drawn without its ligature"
          exit 1
        fi
      done
      for text in '|>' '1..=2'; do
        if [ "$(shaped ${conf ""} "$text")" != "$(shaped ${conf "<include>${features}</include>"} "$text")" ]; then
          echo "$text is drawn as a ligature, whose set is meant to be off"
          exit 1
        fi
      done
      touch $out
    '';
}
