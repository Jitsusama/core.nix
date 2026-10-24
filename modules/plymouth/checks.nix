# The checks of plymouth, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
{
  # The boot's lock is drawn whole: its picture at each scale holds all the
  # ink the glyph has when drawn with room to spare, and is the line's height.
  boot-lock-is-whole =
    let
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        { jitsusama.login.account = "someone"; }
      ];
      inherit (machine.config.boot.plymouth) themePackages;
      theme = (account machine).jitsusama.theme;
    in
    pkgs.runCommandLocal "boot-lock-is-whole"
      {
        nativeBuildInputs = [
          pkgs.imagemagick
          pkgs.pango.bin
        ];
        LC_ALL = "C.UTF-8";
        FONTCONFIG_FILE = pkgs.makeFontsConf { fontDirectories = machine.config.fonts.packages; };
      }
      ''
        ${lib.getExe pkgs.bash} ${../../tests/boot-lock-is-whole.sh} ${
          lib.escapeShellArgs [
            "${lib.head themePackages}/share/plymouth/themes/jitsusama"
            "${theme.font.mono.family} ${toString theme.type.mono.large}"
          ]
        }
        touch $out
      '';

  # The splash keeps systemd's lines off the console, so nothing shows
  # between the firmware's logo and the splash, which a test machine can't
  # show: its kernel writes everything to the screen, and systemd writes to
  # its serial line. A machine without the splash keeps the lines.
  the-splash-keeps-systemd-quiet =
    let
      lines =
        modules:
        !lib.elem "systemd.show_status=error"
          (bareNixos ([ self.nixosModules.graphical ] ++ modules)).config.boot.kernelParams;
    in
    holds "the-splash-keeps-systemd-quiet"
      "systemd writes its lines over the splash, or a machine without one lost them."
      (!lines [ { jitsusama.login.account = "someone"; } ] && lines [ ]);
}
// onLinux {
  # clippy finds nothing in what tells Plymouth of a slow boot or
  # shutdown, its tests included.
  plymouth-waits-passes-clippy = passesClippy "plymouth-waits-passes-clippy" (
    pkgs.callPackage ./waits/package.nix { }
  );
}
