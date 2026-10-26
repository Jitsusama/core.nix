# The checks of console, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  graphical,
  holds,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  self,
  ...
}:
{
  # A failed boot's shell says what stopped it, as tests/boot-stopped.sh
  # says, with stand-ins for systemctl and journalctl. The boot test shows it
  # on a real console, in minutes; this holds what it says, in seconds.
  a-failed-boot-says-what-stopped-it =
    let
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        { jitsusama.login.account = "someone"; }
      ];
      stopped = lib.removePrefix "-" (
        lib.findFirst (lib.hasInfix "boot-stopped") (throw "the emergency shell runs no boot-stopped")
          machine.config.systemd.services.emergency.serviceConfig.ExecStartPre
      );
    in
    pkgs.runCommandLocal "a-failed-boot-says-what-stopped-it" { } ''
      ${lib.getExe pkgs.bash} ${./tests/boot-stopped.sh} ${stopped}
      touch $out
    '';

  # The console wears the theme of the account the machine signs in, in the
  # console's order, and follows it when the theme changes. A machine that
  # names no account keeps the kernel's colours.
  the-console-wears-the-theme =
    let
      wearing =
        theme:
        (bareNixos [
          self.nixosModules.workstation
          self.nixosModules.graphical
          {
            jitsusama.login.account = "someone";
            home-manager.users.someone.jitsusama.theme.name = theme;
          }
        ]).config.console.colors;
      file = theme: builtins.fromTOML (builtins.readFile ../theme/${theme}.toml);
      pigment =
        theme: role:
        lib.toLower (lib.removePrefix "#" (file theme).palette.${(file theme).terminal.${role}});
      follows =
        theme:
        let
          colours = map lib.toLower (wearing theme);
        in
        lib.length colours == 16
        && lib.elemAt colours 0 == pigment theme "black"
        && lib.elemAt colours 1 == pigment theme "red"
        && lib.elemAt colours 15 == pigment theme "bright_white";
    in
    holds "the-console-wears-the-theme"
      "The console's colours aren't the account's theme, in the console's order."
      (follows "osaka-jade" && follows "flexoki-light" && graphical.config.console.colors == [ ]);

  # Above a console's login prompt, the account is greeted by the first word
  # of its description, or by its name without one, as the greeter calls it,
  # with the keys back to the desktop, in place of NixOS's banner, which a
  # machine that names no account keeps.
  the-console-greets-the-account =
    let
      issue =
        description:
        let
          text =
            (bareNixos [
              self.nixosModules.workstation
              self.nixosModules.graphical
              {
                jitsusama.login.account = "someone";
                users.users.someone.description = description;
              }
            ]).config.environment.etc.issue.text;
        in
        if text == null then "" else text;
      greets =
        description: called:
        lib.hasInfix "Hello," (issue description)
        && lib.hasInfix "m${called}" (issue description)
        && lib.hasInfix "Ctrl+Alt+F1" (issue description)
        && !lib.hasInfix "Welcome" (issue description);
    in
    holds "the-console-greets-the-account"
      "A console doesn't greet the account as the greeter calls it, with the keys back to the desktop."
      (
        greets "Sam Rivera" "Sam"
        && greets "" "someone"
        && graphical.config.environment.etc.issue.text == null
      );

  # The console's font is the account's monospace face at Medium, drawn into
  # the kernel's 16 by 32 cell: a PSF 2 font of at most 256 glyphs, so the
  # console keeps its bright colours, naming what each glyph stands for, with
  # slot 32 blank, since the console clears the screen with it and kbd
  # refuses a font without, and loaded in the initrd. A machine that names
  # no account keeps the kernel's own font.
  the-console-wears-the-face =
    let
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        { jitsusama.login.account = "someone"; }
      ];
      inherit (machine.config) console;
      face = (account machine).jitsusama.theme.font.mono.family;
    in
    if !console.earlySetup || graphical.config.console.font != null then
      holds "the-console-wears-the-face"
        "The console's font isn't loaded in the initrd, or a machine without an account has one."
        false
    else
      # tests/console-wears-the-face.sh reads the font.
      pkgs.runCommandLocal "the-console-wears-the-face" { } ''
        ${lib.getExe pkgs.bash} ${./tests/console-wears-the-face.sh} ${
          lib.escapeShellArgs [
            console.font
            face
          ]
        }
        touch $out
      '';

  # The font's build takes only the face asked for at Medium, as
  # tests/console-font.sh says, with stand-ins for fontconfig and the
  # converter: the machine's real fonts show only the face that's there,
  # never what a missing one would do.
  the-console-font-refuses-a-stand-in =
    pkgs.runCommandLocal "the-console-font-refuses-a-stand-in" { }
      ''
        ${lib.getExe pkgs.bash} ${./tests/console-font.sh} ${./font.sh}
        touch $out
      '';
}
// onLinux {
  # clippy finds nothing in the console's font tool, its tests included.
  console-font-passes-clippy = passesClippy "console-font-passes-clippy" (
    pkgs.callPackage ./font/package.nix { }
  );
}
