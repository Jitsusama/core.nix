# The checks of the machine's fonts, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  lib,
  onLinux,
  pkgs,
  self,
  ...
}:
let
  # The graphical role with an account signed in at boot, whose theme the
  # machine's fonts answer to.
  graphical = bareNixos [
    self.nixosModules.workstation
    self.nixosModules.graphical
    { jitsusama.login.account = "someone"; }
  ];
  inherit ((account graphical).jitsusama.theme.font) sans mono;
  # A check run with that machine's fontconfig: its /etc/fonts copied here,
  # its files naming each other here rather than under /etc, and fontconfig
  # pointed at the copy for the files it names without a folder.
  withFontconfig =
    name: script:
    pkgs.runCommandLocal name { nativeBuildInputs = [ pkgs.fontconfig ]; } ''
      mkdir fonts
      etc=${graphical.config.environment.etc.fonts.source}
      cp -rL $etc/fonts.conf $etc/conf.d fonts/
      if [ -e $etc/local.conf ]; then cp -L $etc/local.conf fonts/; fi
      chmod -R u+w fonts
      find fonts -type f -exec sed -i "s|/etc/fonts/|$PWD/fonts/|g" {} +
      export FONTCONFIG_FILE=$PWD/fonts/fonts.conf FONTCONFIG_PATH=$PWD/fonts
      export HOME=$PWD XDG_CACHE_HOME=$PWD/cache
      ${script}
      touch $out
    '';
in
onLinux {
  # Asked for the theme's sans face as its variable font, by the name GitHub's
  # pages ask for first, the machine's fontconfig gives the file with every
  # axis, at its normal width, and lists no other file under that name, while
  # the face asked for by its own name keeps its static files.
  mona-sans-vf-is-the-whole-face = withFontconfig "mona-sans-vf-is-the-whole-face" ''
    whole=${pkgs.mona-sans}/share/fonts/truetype/'MonaSansVF[wdth,wght,opsz,ital].ttf'
    fc-list -f '%{file}\n' '${sans.family} VF' | sort -u > listed
    cat listed
    test "$(cat listed)" = "$whole"
    fc-match -f '%{file} %{width}\n' '${sans.family} VF' > matched
    cat matched
    test "$(cat matched)" = "$whole 100"
    fc-match -f '%{file}\n' '${sans.family}' > theme
    cat theme
    test "$(cat theme)" = ${pkgs.mona-sans}/share/fonts/opentype/MonaSans-Regular.otf
  '';

  # Asked for a Mac's system font or its monospaced one by any name a page
  # gives them, the machine's fontconfig gives the theme's face, carrying the
  # name asked for, which is what Chrome takes a face under. fc-match reads
  # a hyphen as the start of a size, so the names go to it escaped.
  the-macs-faces-are-the-themes =
    let
      asks = face: name: ''
        fc-match -f '%{family}\n' -- ${
          lib.escapeShellArg (lib.replaceStrings [ "-" ] [ "\\-" ] name)
        } > matched
        echo ${lib.escapeShellArg name}: "$(cat matched)"
        grep -qF -- ${lib.escapeShellArg face.family} matched
        grep -qF -- ${lib.escapeShellArg name} matched
      '';
    in
    withFontconfig "the-macs-faces-are-the-themes" (
      lib.concatMapStrings (asks sans) [
        "-apple-system"
        "BlinkMacSystemFont"
        "ui-sans-serif"
      ]
      + lib.concatMapStrings (asks mono) [
        "ui-monospace"
        "SFMono-Regular"
        "SF Mono"
        "Menlo"
      ]
    );
}
