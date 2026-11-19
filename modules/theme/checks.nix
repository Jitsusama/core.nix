# The checks of theme, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  onLinux,
  pkgs,
  self,
  workstation,
  ...
}:
let
  # Whether every pose of one of the margin's animals reads as that one
  # animal (RSCH-20261113-24R2QV's rule 3, with VKCF3Z's c234c1d4): no lit
  # pixel stands alone, a diagonal counting as touching, since a lone pixel
  # at this size reads as dirt, not detail; and each pose's lit pixels are
  # within 15% of its sitting's, so a tuck or a stretch doesn't read as a
  # smaller or bigger animal. Each is held to its own sitting, as a puppy is
  # stockier than the cat. Its strides, in the order it walks them, each
  # step one pair of feet, so its paws are seen walking rather than the
  # animal skidding (Joel, 2026-11-14 17:19: "i always want to see its paws
  # moving when it is walking or running"): around the walk's cycle, from
  # one stride to the next and from the last back to the first, all but the feet on the bottom row
  # and the tail's tip, the top row's last two columns, holds still, and of
  # the feet only the fore half or the hind half changes. Over the whole
  # walk each half changes, since a pair that never steps is being dragged
  # however the other one walks; with feet apart or gathered, that takes
  # four strides or more. Strides are drawn facing left: the fore half is
  # the left one, and the tail's tip is at the top right. What's found, as
  # whether it holds and what it says, for readsAsOne and for the check that
  # each skid is refused; `walkedFrom` is where the strides were read from.
  poseFaults =
    {
      sitting,
      poses,
      strides,
      walkedFrom ? null,
    }:
    let
      glyphs = import ./glyphs.nix { inherit lib; };
      # The sitting is named among the undrawn when it's missing or misnamed,
      # rather than every pose being called unlike a sitting of nothing, and
      # so is a stride, which is held to the stray and size rules too rather
      # than only being called skidding.
      named = lib.unique (poses ++ strides ++ [ sitting ]);
      px = name: glyphs.${name}.pixels or [ ];
      drawn = lib.filter (name: (glyphs.${name}.pixels or [ ]) != [ ]) named;
      missing = lib.subtractLists drawn named;
      # Each lit pixel, as its column and row.
      lit =
        rows:
        lib.concatLists (
          lib.imap0 (
            y: row:
            map (x: { inherit x y; }) (
              lib.filter (x: builtins.substring x 1 row == "#") (lib.range 0 (lib.stringLength row - 1))
            )
          ) rows
        );
      alone =
        cells: at:
        !lib.any (
          other:
          other != at
          && lib.max (other.x - at.x) (at.x - other.x) <= 1
          && lib.max (other.y - at.y) (at.y - other.y) <= 1
        ) cells;
      stray = lib.filter (
        name: lib.any (alone (lit glyphs.${name}.pixels)) (lit glyphs.${name}.pixels)
      ) drawn;
      sat = lib.length (lit (glyphs.${sitting}.pixels or [ ]));
      # Within 15% of sitting's, in whole pixels: 100 times the gap at most 15
      # times sitting's.
      unlike = lib.optionals (sat > 0) (
        lib.filter (
          name:
          let
            gap = lib.length (lit glyphs.${name}.pixels) - sat;
          in
          100 * lib.max gap (-gap) > 15 * sat
        ) drawn
      );
      steps = lib.imap0 (i: from: {
        inherit from;
        to = lib.elemAt strides (lib.mod (i + 1) (lib.length strides));
      }) strides;
      # Which halves of the feet's row, fore and hind, differ between two;
      # on an odd width the middle column is the hind half's, so every
      # column is compared.
      moved =
        a: b:
        let
          width = lib.stringLength (lib.last a);
          half = width / 2;
          feet = rows: {
            fore = builtins.substring 0 half (lib.last rows);
            hind = builtins.substring half (width - half) (lib.last rows);
          };
        in
        lib.filter (pair: (feet a).${pair} != (feet b).${pair}) [
          "fore"
          "hind"
        ];
      # Why a step skids, or null when it steps one pair; an undrawn stride
      # is said among the undrawn instead.
      why =
        { from, to }:
        let
          a = px from;
          b = px to;
          back = rows: builtins.substring 0 (lib.stringLength (lib.head rows) - 2) (lib.head rows);
          body = rows: [ (back rows) ] ++ lib.sublist 1 (lib.length rows - 2) rows;
          pairs = lib.length (moved a b);
        in
        if a == [ ] || b == [ ] then
          null
        else if lib.length a != lib.length b || body a != body b then
          "the body moved"
        else if pairs == 2 then
          "both pairs moved"
        else if pairs == 0 then
          "no pair moved"
        else
          null;
      skidding = map (step: "${step.from} to ${step.to}, ${why step}") (
        lib.filter (step: why step != null) steps
      );
      stepping = pair: lib.any (step: lib.elem pair (moved (px step.from) (px step.to))) steps;
      frozen = lib.optionals (strides != [ ] && lib.all (name: px name != [ ]) strides) (
        lib.filter (pair: !stepping pair) [
          "fore"
          "hind"
        ]
      );
      unread = lib.optionalString (walkedFrom != null) " read from ${walkedFrom}";
    in
    {
      ok =
        lib.all (found: found == [ ]) [
          stray
          unlike
          skidding
          frozen
          missing
        ]
        && strides != [ ];
      said = lib.concatStringsSep ", " (
        [
          "stray ${builtins.toJSON stray}"
          "unlike ${builtins.toJSON unlike}"
          "skidding ${builtins.toJSON skidding}"
          "never stepping ${builtins.toJSON frozen}"
          "undrawn ${builtins.toJSON missing}"
        ]
        ++ lib.optional (strides == [ ]) "no strides${unread}"
      );
    };

  # Whether every pose of one of the margin's animals reads as that one
  # animal, by poseFaults.
  readsAsOne =
    { check, animal, ... }@walk:
    let
      found = poseFaults (
        removeAttrs walk [
          "check"
          "animal"
        ]
      );
    in
    holds check "The ${animal}'s poses must read as one ${animal}: ${found.said}." found.ok;

  # The walk the cat's code takes, read from the code that walks it, so its
  # check proves the walk that's drawn rather than a copy: the words WALKS
  # lists, when it's every list STRIDING strides by. Each is read whole, to
  # its semicolon, spaces aside, and must be only what's meant, so anything
  # else, another list, a stride written inline, an entry not a quoted
  # word, something after its bracket or either not found, reads as no
  # walk, which fails saying where it looked.
  walkIn =
    ways:
    let
      # The constant's whole declaration, up to the semicolon that ends it,
      # without its spaces.
      declared =
        name:
        let
          after = lib.splitString "const ${name} = " ways ++ [ "" ];
        in
        lib.replaceStrings [ " " "\t" "\n" ] [ "" "" "" ] (
          lib.head (lib.splitString ";" (lib.elemAt after 1))
        );
      walks = declared "WALKS";
      striding = declared "STRIDING";
      word = "[\"'][A-Za-z]+[\"']";
      stride = "[A-Za-z_]+:WALKS";
      words = map lib.head (lib.filter lib.isList (builtins.split "[\"']([A-Za-z]+)[\"']" walks));
    in
    lib.optionals (
      builtins.match "\\[(${word},)*${word},?]" walks != null
      && builtins.match "\\{(${stride},)*${stride},?}" striding != null
    ) words;
  catWalk = walkIn (builtins.readFile ../margin-cat/MarginCatWays.js);
  catWalkedFrom = "STRIDING's WALKS in modules/margin-cat/MarginCatWays.js";
in
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
  # margin's cat's own poses are all the margin's cat, so they share the paw,
  # as its yarn's drawings, lying and rolling, are all its yarn, and the
  # margin's puppy's poses are all the puppy, sharing the dog.
  each-mark-means-one-thing =
    let
      glyphs = import ./glyphs.nix { inherit lib; };
      oneMeaning = [
        [
          "batting"
          "blinking"
          "curled"
          "earFlick"
          "falling"
          "grooming"
          "leaning"
          "leaping"
          "lookingAbout"
          "pouncing"
          "scruffed"
          "stretching"
          "tailSwish"
          "tucked"
          "walkingA"
          "walkingB"
          "walkingC"
          "walkingD"
        ]
        [
          "puppyBowing"
          "puppySitting"
          "puppyWalkingA"
          "puppyWalkingB"
          "puppyWalkingC"
          "puppyWalkingD"
        ]
        [
          "heat"
          "temperature"
        ]
        [
          "yarn"
          "yarnRollingA"
          "yarnRollingB"
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

  # Every mark drawn in pixels (the empty places' cat, and the margin's cat
  # and puppy) stands as level as a glyph does (docs/design.md, Marks one
  # height): square, its ink touching its top and bottom rows, and centred
  # across to within a cell, so each pose of the cat, and of the puppy,
  # stands on the same middle line at the same height. The cat's yarn is a
  # thing on the ground beside it, not a mark, so it's held to that
  # instead: drawn in the sitting cat's own square of cells, and standing on
  # their bottom row, the cat's ground line, however low it lies. The three
  # empty places' marks, the margin's cat's and puppy's poses named below
  # and the cat's yarn are drawn, so it can't pass on none.
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
      grounded =
        rows:
        let
          cells = lib.length glyphs.nothingHere.pixels;
        in
        lib.length rows == cells
        && lib.all (row: lib.stringLength row == cells) rows
        && inked (lib.last rows) != [ ];
      stands = name: if lib.hasPrefix "yarn" name then grounded else level;
      crooked = lib.attrNames (lib.filterAttrs (name: mark: !stands name mark.pixels) drawn);
      missing = lib.filter (name: !(drawn ? ${name})) [
        "batting"
        "blinking"
        "curled"
        "earFlick"
        "forYouEmpty"
        "grooming"
        "leaning"
        "leaping"
        "lookingAbout"
        "nothingFound"
        "nothingHere"
        "pouncing"
        "puppyBowing"
        "puppySitting"
        "puppyWalkingA"
        "puppyWalkingB"
        "puppyWalkingC"
        "puppyWalkingD"
        "scruffed"
        "stretching"
        "tailSwish"
        "walkingA"
        "walkingB"
        "walkingC"
        "walkingD"
        "yarn"
        "yarnRollingA"
        "yarnRollingB"
      ];
      said = "crooked ${builtins.toJSON crooked}, undrawn ${builtins.toJSON missing}";
    in
    holds "every-pixel-mark-stands-level" "Pixel marks must be square, level and centred: ${said}." (
      crooked == [ ] && missing == [ ]
    );

  # Every pose of the margin's cat reads as the same cat (readsAsOne). A
  # leap's rise, tuck and fall are drawn, so every fall can take the last.
  every-cat-pose-reads-as-one-cat = readsAsOne {
    check = "every-cat-pose-reads-as-one-cat";
    animal = "cat";
    sitting = "nothingHere";
    strides = catWalk;
    walkedFrom = catWalkedFrom;
    poses = [
      "batting"
      "blinking"
      "curled"
      "earFlick"
      "falling"
      "grooming"
      "leaning"
      "leaping"
      "lookingAbout"
      "nothingHere"
      "pouncing"
      "scruffed"
      "stretching"
      "tailSwish"
      "tucked"
      "walkingA"
      "walkingB"
      "walkingC"
      "walkingD"
    ];
  };

  # Every pose of the margin's puppy reads as the same puppy (readsAsOne),
  # held to its own sitting, as the cat's are to the cat's. Nothing walks it
  # yet, so its strides are listed here in the cat's order, A, C, B, D; once
  # its code walks it, they're read from there, as the cat's are.
  every-puppy-pose-reads-as-one-puppy = readsAsOne {
    check = "every-puppy-pose-reads-as-one-puppy";
    animal = "puppy";
    sitting = "puppySitting";
    strides = [
      "puppyWalkingA"
      "puppyWalkingC"
      "puppyWalkingB"
      "puppyWalkingD"
    ];
    poses = [
      "puppyBowing"
      "puppySitting"
      "puppyWalkingA"
      "puppyWalkingB"
      "puppyWalkingC"
      "puppyWalkingD"
    ];
  };

  # The stride rule refuses each way a walk skids, by its reason, so it's
  # shown refusing as well as passing: the cat's own strides out of order,
  # alone, with a pair never stepping, after a pose that isn't a stride,
  # undrawn, and none at all; and the cat's code read as no walk when it
  # strides by anything but WALKS's quoted words, though as its walk when
  # they're only spaced or quoted otherwise.
  every-skidding-walk-is-refused =
    let
      said =
        strides:
        poseFaults {
          sitting = "nothingHere";
          poses = [ "nothingHere" ];
          walkedFrom = "here";
          inherit strides;
        };
      refused = { strides, reason }: !(said strides).ok && lib.hasInfix reason (said strides).said;
      strides = ''const WALKS = ["walkingA", "walkingC", "walkingB", "walkingD"];'';
      striding = "const STRIDING = { goes: WALKS, climbs: WALKS };";
      walkedOf = ways: {
        strides = walkIn ways;
        reason = "no strides read from here";
      };
      walks = [
        {
          strides = [
            "walkingA"
            "walkingB"
            "walkingC"
            "walkingD"
          ];
          reason = "walkingA to walkingB, both pairs moved";
        }
        {
          strides = [ "walkingA" ];
          reason = "walkingA to walkingA, no pair moved";
        }
        {
          strides = [
            "walkingA"
            "walkingC"
          ];
          reason = "never stepping [\"hind\"]";
        }
        {
          strides = [
            "nothingHere"
            "walkingA"
          ];
          reason = "nothingHere to walkingA, the body moved";
        }
        {
          strides = [
            "walkingA"
            "walkingZ"
          ];
          reason = "undrawn [\"walkingZ\"]";
        }
        (walkedOf "")
        (walkedOf strides)
        (walkedOf striding)
        (walkedOf "${strides}\nconst STRIDING = { goes: WALKS, climbs: [\"walkingA\"] };")
        (walkedOf "${strides}\nconst STRIDING = { goes: WALKS, climbs: WALKS.slice(1) };")
        (walkedOf "${strides}\nconst STRIDING = { goes: WALKS, climbs: RUNS };")
        (walkedOf "const WALKS = [\"walkingA\", ...RUNS];\n${striding}")
        (walkedOf "const WALKS = [\"walkingA\", \"walkingC\"].concat(RUNS);\n${striding}")
        (walkedOf "${strides}\nconst STRIDING = { goes: WALKS, climbs: WALKS } || RUNS;")
      ];
      # The same walk, spaced and quoted otherwise, still reads whole.
      reread = walkIn ''
        const WALKS = [
          'walkingA',
          'walkingC',
          'walkingB',
          'walkingD',
        ];
        ${striding}
      '';
      passed = map (walk: "${walk.reason}, said ${(said walk.strides).said}") (
        lib.filter (walk: !refused walk) walks
      );
    in
    holds "every-skidding-walk-is-refused"
      (
        "Every skidding walk must be refused, by its reason, and a walk spaced otherwise "
        + "read whole: ${builtins.toJSON passed} weren't, and it read ${builtins.toJSON reread}."
      )
      (
        passed == [ ]
        &&
          reread == [
            "walkingA"
            "walkingC"
            "walkingB"
            "walkingD"
          ]
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

  # A program reading theme.json draws a mark the shell turns the way the
  # shell does, so it must be told which turn: the down triangle is the
  # triangle turned, and drawn bare it reads as pointing up. Every other mark
  # says it isn't turned rather than leaving the reader to guess.
  theme-json-says-which-marks-turn =
    let
      home = account workstation;
      inherit ((builtins.fromJSON home.xdg.configFile."jitsusama/theme.json".text)) marks;
      turned = lib.attrNames (lib.filterAttrs (_: mark: mark.turned or null == true) marks);
      unsaid = toString (lib.length (lib.attrNames (lib.filterAttrs (_: m: !(m ? turned)) marks)));
    in
    holds "theme-json-says-which-marks-turn"
      "theme.json turns ${builtins.toJSON turned}, and says nothing of ${unsaid} marks."
      (turned == [ "shapeDownTriangle" ] && unsaid == "0");
}
# What follows runs the Linux workstation's own programs, which a Mac can't run.
// onLinux {
  # The web-design skill's homes probe, run in headless Chromium over small
  # sites of its own (web/homes-test.py): a pattern's names learned from the
  # page's links on the sites alone, each row and record the system reader
  # drops named, a kind's first claim kept, and a shortened name's tail read
  # as no mention. Chromium builds only on Linux.
  homes-probe-reads-as-the-reader =
    let
      python = pkgs.python3.withPackages (python: [
        python.playwright
        python.pytest
      ]);
    in
    pkgs.runCommandLocal "homes-probe-reads-as-the-reader"
      {
        env = {
          WEB_SKILL = "${./web/skill}";
          PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
          PYTHONDONTWRITEBYTECODE = "1";
        };
      }
      ''
        HOME=$TMPDIR ${python}/bin/python -m pytest -p no:cacheprovider \
          --basetemp="$TMPDIR/pytest" ${./web/homes-test.py}
        touch $out
      '';
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
