# The checks of the machine's fonts, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  graphical,
  onLinux,
  pkgs,
  ...
}:
onLinux {
  # Asked for the theme's sans face as its variable font, by the name GitHub's
  # pages ask for first, the machine's fontconfig gives the file with every
  # axis, at its normal width, and lists no other file under that name, while
  # the face asked for by its own name keeps its static files. The machine's
  # /etc/fonts is copied here, its files naming each other here rather than
  # under /etc, and fontconfig is pointed at the copy for the files it names
  # without a folder.
  mona-sans-vf-is-the-whole-face =
    let
      sans = (account graphical).jitsusama.theme.font.sans.family;
    in
    pkgs.runCommandLocal "mona-sans-vf-is-the-whole-face" { nativeBuildInputs = [ pkgs.fontconfig ]; }
      ''
        mkdir fonts
        etc=${graphical.config.environment.etc.fonts.source}
        cp -rL $etc/fonts.conf $etc/conf.d fonts/
        if [ -e $etc/local.conf ]; then cp -L $etc/local.conf fonts/; fi
        chmod -R u+w fonts
        find fonts -type f -exec sed -i "s|/etc/fonts/|$PWD/fonts/|g" {} +
        export FONTCONFIG_FILE=$PWD/fonts/fonts.conf FONTCONFIG_PATH=$PWD/fonts
        export HOME=$PWD XDG_CACHE_HOME=$PWD/cache
        whole=${pkgs.mona-sans}/share/fonts/truetype/'MonaSansVF[wdth,wght,opsz,ital].ttf'
        fc-list -f '%{file}\n' '${sans} VF' | sort -u > listed
        cat listed
        test "$(cat listed)" = "$whole"
        fc-match -f '%{file} %{width}\n' '${sans} VF' > matched
        cat matched
        test "$(cat matched)" = "$whole 100"
        fc-match -f '%{file}\n' '${sans}' > theme
        cat theme
        test "$(cat theme)" = ${pkgs.mona-sans}/share/fonts/opentype/MonaSans-Regular.otf
        touch $out
      '';
}
