# The theme, for a website: a folder a site copies whole, holding the
# theme's colours, type and space as CSS custom properties (theme.css,
# written here from the theme), the defaults every page of Joel's sets on top
# of them (base.css, written by hand beside this file), and the theme's two
# faces cut small enough to send (fonts.py). Before it, every site copied the
# theme's hex values by hand, and each drifted from it: the old text colour
# kept after the theme moved on, a muted made up, full font files of 1.5 MB.
# The web-design skill beside it says how a site uses it.
#
# A site has both a dark and a light look, since a reader picks theirs: the
# machine's theme gives its own side, and core's theme of the other
# appearance gives the other, so Osaka Jade's dark pairs with Flexoki Light.
{
  pkgs,
  cfg,
  themes,
}:
let
  inherit (pkgs) lib;

  # The other side's colours, from the first of core's themes drawn the
  # other way round, each role its pigment.
  other =
    let
      files = map (file: builtins.fromTOML (builtins.readFile file)) (lib.attrValues themes);
      file = lib.findFirst (theme: theme.appearance != cfg.appearance) (throw ''
        A site needs a dark and a light look, and core has no theme drawn
        ${if cfg.appearance == "dark" then "light" else "dark"} to pair with this one.
      '') files;
    in
    lib.mapAttrs (_: pigment: file.palette.${pigment}) file.colours;

  sides =
    if cfg.appearance == "dark" then
      {
        dark = cfg.colours;
        light = other;
      }
    else
      {
        dark = other;
        light = cfg.colours;
      };

  # The roles a tint turns: the grounds, the text on them and the accent,
  # which together are what makes a site read as green or as blue. The hues
  # glyphs wear and the states keep theirs, since each means one thing on
  # every site.
  turned = [
    "background"
    "sunken"
    "deepest"
    "raised"
    "selection"
    "rule"
    "edge"
    "shadow"
    "text"
    "strong"
    "muted"
    "accent"
    "on_accent"
  ];

  property = name: "--${lib.replaceStrings [ "_" ] [ "-" ] name}";

  roles =
    colours: turn:
    lib.concatStrings (
      lib.mapAttrsToList (
        name: value:
        if turn then
          lib.optionalString (lib.elem name turned) "  ${property name}: oklch(from ${value} l c calc(h + var(--turn)));\n"
        else
          "  ${property name}: ${value};\n"
      ) colours
    );

  # A site follows the reader's system unless the reader picked a look on the
  # page, which sets data-theme on the root; without either it's dark, the
  # look of every site Joel has praised.
  scheme = turn: ''
    :root,
    :root[data-theme="dark"] {
      color-scheme: dark;
    ${roles sides.dark turn}}

    @media (prefers-color-scheme: light) {
      :root:not([data-theme="dark"]) {
        color-scheme: light;
    ${roles sides.light turn}  }
    }

    :root[data-theme="light"] {
      color-scheme: light;
    ${roles sides.light turn}}
  '';

  # A number to four places, as CSS reads it.
  fixed =
    x:
    let
      n = builtins.floor (x * 10000 + 0.5);
      whole = n / 10000;
    in
    "${toString whole}.${lib.fixedWidthNumber 4 (n - whole * 10000)}";

  rem = px: "${fixed (px / 16.0)}rem";

  power =
    x: n:
    if n == 0 then
      1.0
    else if n < 0 then
      power x (n + 1) / x
    else
      x * power x (n - 1);

  # A size that grows with the window from its least at a phone's width to
  # its most at a laptop's, in rem so the reader's own font size still counts.
  fluid =
    least: most:
    let
      narrow = 360.0;
      wide = 1280.0;
      slope = (most - least) / (wide - narrow);
    in
    "clamp(${rem least}, ${rem (least - slope * narrow)} + ${fixed (slope * 100)}vw, ${rem most})";

  # The type scale, fluid: the theme's ratio at a laptop's width, and a
  # smaller one at a phone's, so a display step doesn't swamp a narrow screen.
  # A page to read sets its body at 18 to 20 px, the least APCA asks of body
  # text at the theme's contrast; an interface at 15 to 16, near the desktop's.
  ratio = builtins.fromJSON cfg.type.ratio;
  narrowRatio = 1.2;
  steps =
    least: most:
    lib.concatStrings (
      lib.mapAttrsToList (
        name: n: "  --step-${name}: ${fluid (least * power narrowRatio n) (most * power ratio n)};\n"
      ) cfg.type.steps
    );

  # Each level of the type hierarchy (docs/design.md, The Levels) as the
  # settings a rule sets it by: its weight, its serif's softness and wonk,
  # its slant, its tracking and the role it's drawn in.
  levels = lib.concatStrings (
    lib.mapAttrsToList (
      name: level:
      let
        inherit (level) axes;
      in
      ''
        --${name}-face: var(--${level.face});
        --${name}-weight: ${toString (axes.wght or level.weight)};
        --${name}-style: ${if level.italic then "italic" else "normal"};
        --${name}-colour: var(${property level.colour});
        --${name}-tracking: ${toString level.tracking}em;
      ''
      + lib.optionalString (level.strong != null) ''
        --${name}-strong: ${toString level.strong};
      ''
      + lib.optionalString (level.face == "serif") ''
        --${name}-soft: ${toString (axes.SOFT or 50)};
        --${name}-wonk: ${toString (axes.WONK or 0)};
      ''
    ) cfg.type.levels
  );

  space =
    lib.concatStrings (
      lib.mapAttrsToList (name: px: "  --${name}: ${rem (px * 1.0)};\n") (
        removeAttrs cfg.space.rung [ "screen" ]
      )
    )
    + "  --screen: ${fluid (cfg.space.rung.items * 1.0) (cfg.space.rung.screen * 1.0)};\n"
    + "  --huge: ${rem (cfg.space.huge * 1.0)};\n"
    + "  --unit: ${rem (cfg.space.unit * 1.0)};\n";

  # Nerd Fonts' Monaspace, which the desktop sets, has no variable cut; the
  # web takes Monaspace's own variable face of the same name.
  monaspace = {
    "MonaspiceAr Nerd Font" = "Argon";
    "MonaspiceKr Nerd Font" = "Krypton";
    "MonaspiceNe Nerd Font" = "Neon";
    "MonaspiceRn Nerd Font" = "Radon";
    "MonaspiceXe Nerd Font" = "Xenon";
  };
  mono =
    monaspace.${cfg.font.mono.family} or (throw ''
      The web's fonts are cut from Monaspace's variable faces, and the theme's
      monospaced face, ${cfg.font.mono.family}, isn't one of them.
    '');

  # Latin, its punctuation, and the arrows, boxes, shapes and checks marks and
  # tables draw with.
  unicodes = lib.concatStringsSep "," [
    "U+0000-00FF"
    "U+0131"
    "U+0152-0153"
    "U+02BB-02BC"
    "U+02C6"
    "U+02DA"
    "U+02DC"
    "U+2000-206F"
    "U+2074"
    "U+20AC"
    "U+2122"
    "U+2190-21FF"
    "U+2212"
    "U+2215"
    "U+2500-259F"
    "U+25A0-25FF"
    "U+2713"
    "U+2717"
  ];

  # Each file, the face it's cut from and the axes pinned. Monaspace keeps
  # its weights and loses its widths, and its slant is cut in two at the
  # ends, so a page that sets no slanted code never fetches the second.
  fonts = {
    "fraunces.woff2" = {
      source = "${pkgs.fraunces}/share/fonts/truetype/Fraunces[SOFT,WONK,opsz,wght].ttf";
      pins = [ ];
    };
    "fraunces-italic.woff2" = {
      source = "${pkgs.fraunces}/share/fonts/truetype/Fraunces-Italic[SOFT,WONK,opsz,wght].ttf";
      pins = [ ];
    };
    "monaspace.woff2" = {
      source = "${pkgs.monaspace}/share/fonts/truetype/Monaspace ${mono} Var.ttf";
      pins = [
        "wdth=100"
        "slnt=0"
      ];
    };
    "monaspace-italic.woff2" = {
      source = "${pkgs.monaspace}/share/fonts/truetype/Monaspace ${mono} Var.ttf";
      pins = [
        "wdth=100"
        "slnt=-11"
      ];
    };
  };

  python = pkgs.python3.withPackages (packages: [
    packages.fonttools
    packages.brotli
  ]);

  serif = cfg.font.serif.family;

  # The faces, and a stand-in for each measured to take the same room, so the
  # page doesn't move when the real face arrives: Fraunces against Times's
  # metrics, Monaspace against Courier's.
  faces = ''
    @font-face {
      font-family: "${serif}";
      font-style: normal;
      font-weight: 100 900;
      font-display: swap;
      src: url("fonts/fraunces.woff2") format("woff2");
    }

    @font-face {
      font-family: "${serif}";
      font-style: italic;
      font-weight: 100 900;
      font-display: swap;
      src: url("fonts/fraunces-italic.woff2") format("woff2");
    }

    @font-face {
      font-family: "Monaspace ${mono}";
      font-style: normal;
      font-weight: 200 800;
      font-display: swap;
      src: url("fonts/monaspace.woff2") format("woff2");
    }

    @font-face {
      font-family: "Monaspace ${mono}";
      font-style: italic;
      font-weight: 200 800;
      font-display: swap;
      src: url("fonts/monaspace-italic.woff2") format("woff2");
    }

    @font-face {
      font-family: "${serif} Stand-in";
      src: local("Times New Roman"), local("Liberation Serif"), local("Tinos");
      size-adjust: 115%;
      ascent-override: 85.1%;
      descent-override: 22.2%;
      line-gap-override: 0%;
    }

    @font-face {
      font-family: "Monaspace Stand-in";
      src: local("Courier New"), local("Liberation Mono"), local("Cousine");
      size-adjust: 103.3%;
      ascent-override: 91.5%;
      descent-override: 19.4%;
      line-gap-override: 9.7%;
    }
  '';

  css = ''
    /* ${cfg.title}, for a website: written by core.nix's theme module from the
       theme, so never edited by hand. A site sets its own rules after it and
       names these properties, never a colour, a size or a face. */

    ${faces}
    ${scheme false}
    /* A site's tint: --turn turns the grounds, the text and the accent round
       the hue wheel by that many degrees, keeping their lightness, so a blue
       site reads as blue and still passes as the theme does. */
    @supports (color: oklch(from red l c h)) {
    ${scheme true}}

    :root {
      --turn: 0;
      /* What a site marks its emphasis in: the action's hue, gold, unless the
         site takes another glyph hue as its own. */
      --tint: var(--action);

      --serif: "${serif}", "${serif} Stand-in", serif;
      --mono: "Monaspace ${mono}", "Monaspace Stand-in", ui-monospace, monospace;
      --mono-features: ${
        lib.concatMapStringsSep ", " (feature: ''"${feature}"'') ([ "calt" ] ++ cfg.font.mono.features)
      };

    ${steps 18.0 20.0}
    ${levels}
    ${space}
      --radius: ${toString cfg.shape.radius}px;
      --border: ${toString cfg.shape.border}px;

      --ease: cubic-bezier(${lib.concatStringsSep ", " cfg.motion.curve});
      --leave: ${toString cfg.motion.short}ms;
      --arrive: ${toString cfg.motion.medium}ms;
      --large: ${toString cfg.motion.long}ms;
    }

    /* An interface to work in sets its body smaller than a page to read. */
    :root[data-kind="interface"] {
    ${steps 15.0 16.0}}
  '';
in
pkgs.runCommandLocal "web-kit"
  {
    inherit css;
    passAsFile = [ "css" ];
  }
  ''
    mkdir -p $out/fonts
    cp $cssPath $out/theme.css
    cp ${./base.css} $out/base.css
    ${lib.concatStrings (
      lib.mapAttrsToList (file: font: ''
        ${python}/bin/python3 ${./fonts.py} ${lib.escapeShellArg font.source} $out/fonts/${file} \
          ${unicodes} ${lib.escapeShellArgs font.pins}
      '') fonts
    )}
  ''
