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
  inherit ((account graphical).jitsusama.theme.font)
    sans
    mono
    serif
    script
    ;
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

  # Asked for the theme's serif face, the machine's fontconfig lists its two
  # variable files and no static cut, and gives the italic one when the
  # italic is asked for, so the theme's axes are set on the file that has
  # them.
  the-serif-is-its-variable-files = withFontconfig "the-serif-is-its-variable-files" ''
    fc-list -f '%{file}\n' '${serif.family}' | sort -u > listed
    cat listed
    test "$(wc -l < listed)" = 2
    ! grep -v -F '[SOFT,WONK,opsz,wght].ttf' listed
    fc-match -f '%{file}\n' '${serif.family}:slant=100' > matched
    cat matched
    grep -q -- '-Italic\[' matched
  '';

  # Asked for a letter of Hebrew, Arabic or CJK in any of the theme's faces,
  # none of which has them, the machine's fontconfig, with the account's own
  # beside it, gives the script face's family for that script, in its order,
  # and not the machine's default sans, which they fell back to before.
  # English, the locale's, is asked for too, as a program's fallback asks
  # for it, since a face that covers it would otherwise outrank the script
  # face.
  the-scripts-are-the-script-faces =
    let
      inherit ((account graphical).fonts.fontconfig) configFile;
      copies = file: ''
        cp ${file.source} config/fontconfig/conf.d/${file.target}
      '';
      falls = face: letter: family: ''
        fc-match -f '%{family[0]}\n' ${lib.escapeShellArg "${face.family}:charset=${letter}:lang=en"} > matched
        echo ${lib.escapeShellArg "${face.family} ${letter}"}: "$(cat matched)"
        test "$(cat matched)" = ${lib.escapeShellArg family}
      '';
      each =
        face:
        lib.concatStrings (
          lib.zipListsWith (falls face) [
            "05d0"
            "0627"
            "4e2d"
          ] script.families
        );
    in
    withFontconfig "the-scripts-are-the-script-faces" (
      ''
        mkdir -p config/fontconfig/conf.d
        ${copies configFile.default-fonts}
        ${copies configFile.script}
        export XDG_CONFIG_HOME=$PWD/config
        # What a failure is read by: which of the account's files were read,
        # and which faces have the letters at all.
        fc-conflist | grep -F "$PWD/config/" || echo "none of the account's files were read"
        for letter in 05d0 0627 4e2d; do
          echo "$letter in: $(fc-list -f '%{family[0]}, ' ":charset=$letter")"
        done
      ''
      + lib.concatMapStrings each [
        sans
        mono
        serif
      ]
    );

  # Asked for a Mac's system font or its monospaced one by any name a page
  # gives them, the machine's fontconfig gives the theme's face, carrying the
  # name asked for, which is what Chrome takes a face under. fc-match reads
  # a hyphen in a bare name as the start of a size, and escapes a leading
  # one differently from one version to the next, so each name is asked for
  # as the family's value, as Chrome hands it over.
  the-macs-faces-are-the-themes =
    let
      asks = face: name: ''
        fc-match -f '%{family}\n' ${lib.escapeShellArg ":family=${name}"} > matched
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
