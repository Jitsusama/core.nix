# The checks of kmscon, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  graphical,
  holds,
  lib,
  self,
  ...
}:
{
  # The consoles signed in on are kmscon's, in the account's monospace face
  # at the terminal's size, with the terminal's padding and colours, all
  # times the screen's scale, and the terminal's keys for larger and smaller
  # text. A machine that names no account keeps the kernel's console.
  the-sign-in-consoles-wear-the-theme =
    let
      signingIn =
        extra:
        (bareNixos (
          [
            self.nixosModules.workstation
            self.nixosModules.graphical
            { jitsusama.login.account = "someone"; }
          ]
          ++ extra
        )).config;
      plain = signingIn [ ];
      laptop = signingIn [ self.nixosModules.dell-xps-14-da14260 ];
      home = plain.home-manager.users.someone.jitsusama.theme;
      settings = machine: machine.services.kmscon.config;
      rgb =
        colour:
        lib.concatMapStringsSep ", " (at: toString (lib.fromHexString (lib.substring at 2 colour))) [
          1
          3
          5
        ];
    in
    holds "the-sign-in-consoles-wear-the-theme"
      "The sign-in consoles aren't kmscon in the account's face, size, padding and colours."
      (
        !graphical.config.services.kmscon.enable
        && plain.services.kmscon.enable
        && lib.hasInfix "padding.patch" (
          lib.concatStringsSep " " (map toString plain.services.kmscon.package.patches)
        )
        && (settings plain).font-engine == "pango"
        && (settings plain).font-name == home.font.mono.family
        && (settings plain).font-size == 14
        && (settings plain).padding-x == home.space.xl
        && (settings plain).padding-y == home.space.l
        && (settings laptop).font-size == 28
        && (settings laptop).padding-x == 2 * home.space.xl
        && (settings laptop).padding-y == 2 * home.space.l
        && (settings plain).palette-background == rgb home.terminal.background
        && (settings plain).palette-foreground == rgb home.terminal.foreground
        && (settings plain).palette-white == rgb home.terminal.bright_white
        && (settings plain).grab-zoom-in == "<Ctrl><Shift>plus"
        && (settings plain).grab-rotate-cw == ""
      );
}
