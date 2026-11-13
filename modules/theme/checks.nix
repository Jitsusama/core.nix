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
  marks-past-the-basic-plane =
    let
      nerd = import ./nerd.nix { inherit lib; };
    in
    holds "marks-past-the-basic-plane" "A mark past U+FFFF came out as two characters." (
      nerd "f075d" != nerd "f075" + "d" && builtins.stringLength (nerd "f075d") == 4
    );

  # No two marks share a glyph unless they mean one thing, so a mark never
  # asks Joel to remember which of two meanings it has here (Joel, 2026-11-07:
  # "icon pairs: we need them to be obvious and not require memory"). Heat
  # and temperature both say how hot, so they share the thermometer, and the
  # margin's cat's own poses are all the margin's cat, so they share the paw.
  each-mark-means-one-thing =
    let
      glyphs = import ./glyphs.nix { inherit lib; };
      oneMeaning = [
        [
          "batting"
          "blinking"
          "curled"
          "earFlick"
          "grooming"
          "lookingAbout"
          "pouncing"
          "stretching"
          "tailSwish"
          "walkingA"
          "walkingB"
        ]
        [
          "heat"
          "temperature"
        ]
      ];
      sharing = lib.filter (names: lib.length names > 1) (
        lib.attrValues (lib.groupBy (name: glyphs.${name}.glyph) (lib.attrNames glyphs))
      );
      twoMeanings = lib.filter (names: !lib.elem (lib.sort lib.lessThan names) oneMeaning) sharing;
    in
    holds "each-mark-means-one-thing"
      "These marks share a glyph but mean different things: ${builtins.toJSON twoMeanings}."
      (twoMeanings == [ ]);

  # Every mark drawn in pixels (the empty places' cat) stands as level as a
  # glyph does (docs/design.md, Marks one height): square, its ink touching
  # its top and bottom rows, and centred across to within a cell, so each
  # pose of the cat stands on the same middle line at the same height. The
  # three empty places' marks, the margin's cat's own poses and its yarn
  # are drawn, so it can't pass on none.
  every-pixel-mark-stands-level =
    let
      glyphs = import ./glyphs.nix { inherit lib; };
      drawn = lib.filterAttrs (_: mark: (mark.pixels or [ ]) != [ ]) glyphs;
      # Where along a row its ink is.
      inked =
        row: lib.filter (at: builtins.substring at 1 row == "#") (lib.range 0 (lib.stringLength row - 1));
      level =
        rows:
        let
          cells = lib.length rows;
          across = lib.concatMap inked rows;
          left = lib.foldl' lib.min cells across;
          right = cells - 1 - lib.foldl' lib.max 0 across;
        in
        lib.all (row: lib.stringLength row == cells) rows
        && inked (lib.head rows) != [ ]
        && inked (lib.last rows) != [ ]
        && lib.max left right - lib.min left right <= 1;
      crooked = lib.attrNames (lib.filterAttrs (_: mark: !level mark.pixels) drawn);
      missing = lib.filter (name: !(drawn ? ${name})) [
        "batting"
        "blinking"
        "curled"
        "earFlick"
        "forYouEmpty"
        "grooming"
        "lookingAbout"
        "nothingFound"
        "nothingHere"
        "pouncing"
        "stretching"
        "tailSwish"
        "walkingA"
        "walkingB"
        "yarn"
      ];
      said = "crooked ${builtins.toJSON crooked}, undrawn ${builtins.toJSON missing}";
    in
    holds "every-pixel-mark-stands-level" "Pixel marks must be square, level and centred: ${said}." (
      crooked == [ ] && missing == [ ]
    );

  # A theme that draws text too faint to read doesn't build, by WCAG's ratio
  # or by APCA's lightness contrast. WCAG's black on white is 21:1, and
  # #777777 on white just misses 4.5:1; Osaka Jade's own muted, at 2.90:1, is
  # the faint text it was made to catch. APCA's black on white is Lc 106.0
  # and white on black 107.9, its own reference values; Osaka Jade's old
  # text, parchment, passes 4.5:1 yet is Lc 67.6, too dim for body text.
  a-faint-theme-is-refused =
    let
      contrast = import ./contrast.nix { inherit lib; };
      apca = import ./apca.nix { inherit lib; };
      refused =
        colours:
        map (check: check.message) (
          lib.filter (check: !check.assertion)
            (account (bareNixos [
              {
                home-manager.users.someone = {
                  imports = [ ./home.nix ];
                  jitsusama.theme = { inherit colours; };
                };
              }
            ])).assertions
        );
      faint = refused { muted = "#53685B"; };
      dim = refused { text = "#C1C497"; };
      faintMark = refused { person = "#D2689C"; };
      # Reads on the ground but not on a selected row, as floating surfaces
      # draw one.
      faintSelected = refused { person = "#D77AA8"; };
      # A warning's and an alert's marks are held as every mark is, Osaka
      # Jade's old red among them, so a problem never reads fainter than
      # the rows around it.
      faintAlert = refused { alert = "#FF5345"; };
      faintWarning = refused { warning = "#FF5345"; };
    in
    holds "a-faint-theme-is-refused" "A theme with text too faint to read wasn't refused." (
      contrast.show (contrast.ratio "#000000" "#ffffff") == "21.00"
      && contrast.show (contrast.ratio "#777777" "#ffffff") == "4.47"
      && lib.any (lib.hasInfix "muted on background: 2.90:1") faint
      && apca.show (apca.lc "#000000" "#ffffff") == "106.0"
      && apca.show (apca.lc "#ffffff" "#000000") == "107.9"
      && apca.lc "#ffffff" "#000000" < 0
      && lib.any (lib.hasInfix "text on background: Lc 67.6, under 75") dim
      && lib.any (lib.hasInfix "person on background: Lc 39.5, under 45") faintMark
      && lib.any (lib.hasInfix "person on a selected row: Lc 42.1, under 45") faintSelected
      && !lib.any (lib.hasInfix "person on background") faintSelected
      && lib.any (lib.hasInfix "alert on background: Lc 42.5, under 45") faintAlert
      && lib.any (lib.hasInfix "alert on a selected row: Lc 39.2, under 45") faintAlert
      && lib.any (lib.hasInfix "warning on a selected row: Lc 39.2, under 45") faintWarning
      && refused { } == [ ]
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
      theme.overrides.text == home.jitsusama.theme.colours.text
      && lib.all (colour: builtins.match "#[0-9a-fA-F]{6}" colour != null) colours
      && mix 0.5 "#000000" "#ffffff" == "#808080"
      && mix 0.25 "#ff0000" "#000000" == "#400000"
    );

  # Picking another theme changes everything the account draws: the desktop
  # rendered in Flexoki Light keeps none of Osaka Jade's colours anywhere in
  # its files, so no surface was left behind in the old one. The design
  # document installed for agents beside the aesthetic skill is passed over:
  # it describes themes, Osaka Jade among them, and draws nothing. So is the
  # web kit's theme.css, which carries the dark look beside the light one on
  # purpose, for a site's reader to choose; the-web-kit-carries-the-theme
  # proves its own side follows the machine's theme.
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
      test ${lib.escapeShellArg home.jitsusama.theme.colours.background} = '#FFFCF0'
      if rg --follow --ignore-case --fixed-strings --line-number --no-heading --hidden \
        --glob '!**/skills/aesthetic/design.md' \
        --glob '!**/jitsusama/web/theme.css' \
        ${
          lib.concatMapStringsSep " " (colour: "-e ${lib.escapeShellArg colour}") (lib.attrValues osakaJade)
        } ${home.home-files}/; then
        echo "The files above kept Osaka Jade's colours when the theme changed."
        exit 1
      fi
      touch $out
    '';

  # The web kit a site copies holds every role of both looks at the theme's
  # own values, the machine's in its own scheme and core's other theme in the
  # other, so a site that names the properties can never drift from the
  # theme as the hand-copied sites did; and it sends a first view's three
  # faces, the upright, the italic a heading's turn draws and the mono, in
  # under 350 KB, the budget the web-design skill sets, so cutting them can't
  # silently grow back to the 1.5 MB of whole font files.
  the-web-kit-carries-the-theme =
    let
      home = account workstation;
      kit = home.xdg.dataFile."jitsusama/web".source;
      flexoki =
        let
          theme = builtins.fromTOML (builtins.readFile ./flexoki-light.toml);
        in
        lib.mapAttrs (_: pigment: theme.palette.${pigment}) theme.colours;
      declares =
        colours:
        lib.concatMapStringsSep " " (
          role: lib.escapeShellArg "--${lib.replaceStrings [ "_" ] [ "-" ] role}: ${colours.${role}};"
        ) (lib.attrNames colours);
    in
    pkgs.runCommandLocal "the-web-kit-carries-the-theme" { } ''
      dark=$(sed -n '/^:root,$/,/^}/p' ${kit}/theme.css)
      light=$(sed -n '/^:root\[data-theme="light"\] {$/,/^}/p' ${kit}/theme.css)
      for line in ${declares home.jitsusama.theme.colours}; do
        grep -qF -- "$line" <<<"$dark" || { echo "The dark look lacks $line"; exit 1; }
      done
      for line in ${declares flexoki}; do
        grep -qF -- "$line" <<<"$light" || { echo "The light look lacks $line"; exit 1; }
      done
      first=$(( $(stat -c %s ${kit}/fonts/fraunces.woff2) \
        + $(stat -c %s ${kit}/fonts/fraunces-italic.woff2) \
        + $(stat -c %s ${kit}/fonts/monaspace.woff2) ))
      if [ "$first" -gt 358400 ]; then
        echo "A first view's faces come to $first bytes, past 350 KB."
        exit 1
      fi
      touch $out
    '';
}
