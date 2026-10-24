# The checks of theme, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  pkgs,
  self,
  workstation,
  ...
}:
{
  # Most of the icon font's marks lie past U+FFFF, which a single JSON escape
  # can't reach: written as one, U+F075D read as U+F075 and a stray "d".
  icon-marks-past-the-basic-plane =
    let
      nerd = import ./nerd.nix { inherit lib; };
    in
    holds "icon-marks-past-the-basic-plane" "An icon mark past U+FFFF came out as two characters." (
      nerd "f075d" != nerd "f075" + "d" && builtins.stringLength (nerd "f075d") == 4
    );

  # A theme that draws text too faint to read doesn't build. The ratio is
  # WCAG's: black on white is 21:1, and #777777 on white just misses 4.5:1.
  # Osaka Jade's own muted, at 2.90:1, is the faint text it was made to catch.
  a-faint-theme-is-refused =
    let
      contrast = import ./contrast.nix { inherit lib; };
      machine = bareNixos [
        {
          home-manager.users.someone = {
            imports = [ ./home.nix ];
            jitsusama.theme.roles.muted = "#53685B";
          };
        }
      ];
      refusals = map (check: check.message) (
        lib.filter (check: !check.assertion) (account machine).assertions
      );
    in
    holds "a-faint-theme-is-refused" "A theme with text too faint to read wasn't refused." (
      contrast.show (contrast.ratio "#000000" "#ffffff") == "21.00"
      && contrast.show (contrast.ratio "#777777" "#ffffff") == "4.47"
      && lib.any (lib.hasInfix "muted on background: 2.90:1") refusals
    );

  # Claude Code's theme is drawn from the machine's, every colour a hex one
  # its format reads, and a diff's tint sits between the background and the
  # colour it's a tint of.
  claude-wears-the-theme =
    let
      home = account workstation;
      theme = builtins.fromJSON home.home.file.".claude/themes/jitsusama.json".text;
      colours = lib.attrValues theme.overrides;
      mix = import ./mix.nix { inherit lib; };
    in
    holds "claude-wears-the-theme" "Claude Code's theme isn't the machine's, or mixes wrong." (
      theme.overrides.text == home.jitsusama.theme.roles.text
      && lib.all (colour: builtins.match "#[0-9a-fA-F]{6}" colour != null) colours
      && mix 0.5 "#000000" "#ffffff" == "#808080"
      && mix 0.25 "#ff0000" "#000000" == "#400000"
    );

  # Picking another theme changes everything the account draws: the desktop
  # rendered in Flexoki Light keeps none of Osaka Jade's colours anywhere in
  # its files, so no surface was left behind in the old one. The design
  # document installed for agents beside the aesthetic skill is passed over:
  # it describes themes, Osaka Jade among them, and draws nothing.
  a-theme-changes-everything =
    let
      osakaJade = (builtins.fromTOML (builtins.readFile ./osaka-jade.toml)).palette;
      machine = bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        { home-manager.users.someone.jitsusama.theme.name = "flexoki-light"; }
      ];
      home = account machine;
    in
    pkgs.runCommandLocal "a-theme-changes-everything" { nativeBuildInputs = [ pkgs.ripgrep ]; } ''
      test ${lib.escapeShellArg home.jitsusama.theme.roles.background} = '#FFFCF0'
      if rg --follow --ignore-case --fixed-strings --line-number --no-heading --hidden \
        --glob '!**/skills/aesthetic/design.md' \
        ${
          lib.concatMapStringsSep " " (colour: "-e ${lib.escapeShellArg colour}") (lib.attrValues osakaJade)
        } ${home.home-files}/; then
        echo "The files above kept Osaka Jade's colours when the theme changed."
        exit 1
      fi
      touch $out
    '';
}
