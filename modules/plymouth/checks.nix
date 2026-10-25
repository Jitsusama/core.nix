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

  # What the boot and the shutdown tell the splash of the generation, read
  # from each shape of entry name systemd-boot can say it chose: lanzaboote's,
  # a specialisation of one, NixOS's own named by a hash, and none at all.
  # The variable is laid out as efivarfs gives it.
  the-splash-reads-the-generation-from-the-entry =
    let
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        { jitsusama.login.account = "someone"; }
      ];
      inherit (machine.config) boot systemd;
      selected = "/sys/firmware/efi/efivars/LoaderEntrySelected-4a67b082-0a4c-41cf-b6c7-440b29bb8c4f";
      standIn =
        script:
        pkgs.writeText "tells-the-generation" (
          lib.replaceStrings
            [ selected (lib.getExe' boot.plymouth.package "plymouth") ]
            [ "\"$selected\"" "plymouth" ]
            script
        );
      # A stand-in for Plymouth's client, keeping what it was told.
      client = pkgs.writeShellScriptBin "plymouth" ''
        printf '%s\n' "$*" > "$told"
      '';
    in
    pkgs.runCommandLocal "the-splash-reads-the-generation-from-the-entry" { } ''
      PATH=${client}/bin:$PATH
      export told=$PWD/told

      chose() {
        printf '\007\000\000\000' > chosen
        printf '%s' "$1" | sed 's/./&\x00/g' >> chosen
        printf '\000\000' >> chosen
      }
      tells() {
        : > told
        selected=$entry ${pkgs.runtimeShell} -e "$script"
        echo "$1: $(cat told)"
        [ "$(cat told)" = "$2" ]
      }

      for script in ${standIn boot.initrd.systemd.services.plymouth-generation.script} \
        ${standIn systemd.services.plymouth-generation.script}; do
        entry=$PWD/chosen
        chose nixos-generation-192-glbpaskycnksgtrtgjna65343gjlq4bdxsvhwx6ug44f.efi
        tells "lanzaboote's" 'update --status=generation:192'
        chose nixos-generation-3-specialisation-work-glbpaskycnksgtrtgjna6534.efi
        tells "a specialisation's" 'update --status=generation:3'
        chose nixos-fc7ba0624fad4bfbf3081d51b380481d75a52f7d2c4c776a21a6cdd4f9.conf
        tells "NixOS's own" ""
        chose nixos-generation-.conf
        tells "one with no number" ""
        entry=$PWD/none
        tells "no entry" ""
      done
      touch $out
    '';
}
// onLinux {
  # clippy finds nothing in what tells Plymouth of a slow boot or
  # shutdown, its tests included.
  plymouth-waits-passes-clippy = passesClippy "plymouth-waits-passes-clippy" (
    pkgs.callPackage ./waits/package.nix { }
  );
}
