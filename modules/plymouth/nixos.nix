# Plymouth, which holds the screen from the initrd until the greeter takes it,
# in the theme the machine's own account wears, as the console and greeter
# are (../console/nixos.nix, ../greetd/nixos.nix). It asks for the disk's PIN
# as the lock screen asks for a password: the lock screen's picture, the
# machine's name where the time goes, and the same panel in the same place
# (theme.script). Every word but a message the boot sends is drawn here, at
# one and two device pixels to a logical one, in the theme's faces and with
# its shadows, so the initrd carries pictures rather than fonts and what the
# screen shows is exactly what was drawn. A machine without such an account,
# or without a theme for it, keeps the console.
{ wallpapers }:
{
  config,
  lib,
  options,
  pkgs,
  ...
}:
let
  account = if options ? jitsusama.login.account then config.jitsusama.login.account else null;
  home =
    if account != null && options ? home-manager then
      config.home-manager.users.${account} or null
    else
      null;
  theme = lib.attrByPath [ "jitsusama" "theme" ] null home;
  inherit (theme) roles;

  number = value: builtins.fromJSON (toString value);
  # Two hex digits of opacity, for a colour ImageMagick reads as #rrggbbaa.
  alpha =
    opacity:
    lib.fixedWidthString 2 "0" (
      lib.toLower (lib.toHexString (builtins.floor (number opacity * 255 + 0.5)))
    );
  # A colour as the script's three numbers from 0 to 1.
  channels =
    hex:
    let
      channel = at: toString (lib.fromHexString (lib.substring at 2 hex) / 255.0);
    in
    "${channel 1}, ${channel 3}, ${channel 5}";

  picture =
    let
      name = theme.wallpaper.lock;
    in
    if name == null then
      null
    else if lib.hasPrefix "/" name then
      name
    else
      "${wallpapers}/${name}";

  # The prompt's place, as Lock.qml and Prompt.qml build it: a field a third
  # of the large step tall, inset and framed, a large glyph a step in from
  # its edge, what's typed a step after the glyph, and a hint just clear of
  # the caret.
  inherit (theme) space;
  frame = theme.shape.border + theme.shape.inset;
  panel = {
    width = 400;
    height = space.xxxl + 2 * frame;
  };
  # How far a shadow can reach, its offset and twice its blur, past which a
  # blur's tail is too faint to see.
  farthest = lib.foldl' lib.max 0;
  reach = farthest (map (shadow: shadow.offset + 2 * shadow.blur) theme.floating.shadows);
  pad = farthest (
    map (shadow: lib.max (lib.max shadow.x shadow.y) 0 + 2 * shadow.blur) theme.wallpaper.shadows
  );
  points = face: step: number theme.type.${face}.${step};
  mono = theme.font.mono.family;
  sans = theme.font.sans.family;

  # The words under the name, keyed by the picture that says them.
  under = {
    starting = "starting";
    locked = "the disk is locked";
    restarting = "restarting";
    stopping = "shutting down";
  };
  hints = {
    pin = "PIN";
    passphrase = "passphrase";
    recovery = "recovery key";
    again = "try again";
  };

  # Draws every picture at a scale, in device pixels: words with pango, the
  # engine Plymouth would use, and their shadows and the panel with
  # ImageMagick.
  drawnAt =
    scale:
    let
      at = n: toString (n * scale);
      file = name: "$out/${name}-${toString scale}.png";
      large = points "mono" "large";
      blurred =
        shadow:
        let
          sigma = toString (shadow.blur * scale / 2.0);
        in
        lib.optionalString (shadow.blur > 0) "-blur 0x${sigma}";
      hint = key: text: words (file "hint-${key}") mono large roles.muted 0 text;
      words = out: face: size: colour: margin: text: ''
        pango-view -q --no-display --dpi=${toString (96 * scale)} --margin=${at margin} \
          --antialias=gray --hinting=none --background=transparent \
          --foreground=${lib.escapeShellArg colour} \
          --font=${lib.escapeShellArg "${face} ${toString size}"} \
          --output=${out} --text=${lib.escapeShellArg text}
      '';
      # Each shadow, the farthest first, under the words: their outline
      # blurred, darkened, coloured and moved.
      lifted = out: face: size: text: ''
        ${words "words.png" face size roles.text pad text}
        magick words.png ${
          lib.concatMapStringsSep " "
            (shadow: ''
              \( -clone 0 -channel A ${blurred shadow} \
                -evaluate multiply ${toString (number shadow.opacity)} +channel \
                -fill '${roles.shadow}' -colorize 100 -roll +${at shadow.x}+${at shadow.y} \)'')
            (lib.reverseList theme.wallpaper.shadows)
        } \( -clone 0 \) -delete 0 -background none -flatten ${out}
      '';
      # The panel and its shadows, the farthest first, on a canvas with room
      # for them, the frame's hairline drawn on its own pixels.
      framed =
        out: edge:
        let
          size = "${at (panel.width + 2 * reach)}x${at (panel.height + 2 * reach)}";
          box =
            inset: dy:
            let
              left = toString (reach * scale + inset);
              top = toString (reach * scale + inset + dy);
              right = toString ((reach + panel.width) * scale - inset - 1);
              bottom = toString ((reach + panel.height) * scale - inset - 1 + dy);
              radius = toString (theme.shape.radius * scale - inset);
            in
            "roundrectangle ${left},${top} ${right},${bottom} ${radius},${radius}";
          half = theme.shape.border * scale / 2.0 - 0.5;
        in
        ''
          magick -size ${size} xc:none ${
            lib.concatMapStringsSep " "
              (shadow: ''
                \( -size ${size} xc:none -fill '${roles.shadow}${alpha shadow.opacity}' \
                  -draw '${box 0 (shadow.offset * scale)}' ${blurred shadow} \)'')
              (lib.reverseList theme.floating.shadows)
          } \( -size ${size} xc:none -fill '${roles.deepest}${alpha theme.floating.opacity}' \
              -stroke '${edge}' -strokewidth ${at theme.shape.border} -draw '${box half 0}' \) \
            -background none -flatten ${out}
        '';
    in
    ''
      ${lifted (file "name") mono (points "mono" "hero") config.networking.hostName}
      ${lib.concatStrings (
        lib.mapAttrsToList (key: text: lifted (file key) sans (points "sans" "large") text) under
      )}
      ${lifted (file "about") mono (points "mono" "small") about}
      ${framed (file "panel") roles.accent}
      ${framed (file "panel-urgent") roles.alert}
      ${words (file "lock") mono large roles.machine 0 lockGlyph}
      ${lib.concatStrings (lib.mapAttrsToList hint hints)}
      ${words (file "dot") mono large roles.strong 0 "●"}
      # The caret in the accent, as tall as a line and two pixels wide.
      magick ${file "dot"} -fill '${roles.accent}' -colorize 100 \
        -channel A -fx 1 +channel -crop ${at 2}x+0+0 +repage ${file "caret"}
    '';

  # What's booting, small at the bottom left.
  about =
    let
      inherit (config.system.nixos) release;
      inherit (config.boot.kernelPackages.kernel) version;
    in
    "NixOS ${release}  ·  Linux ${version}";
  lockGlyph = builtins.fromJSON ''"\uf023"'';

  script = pkgs.replaceVars ./theme.script {
    inherit pad reach;
    deepest = channels roles.deepest;
    textColour = channels roles.text;
    sansFamily = sans;
    sansLarge = toString (points "sans" "large");
    apart = space.xl;
    said = space.xl;
    panelWidth = panel.width;
    panelHeight = panel.height;
    glyphLeft = frame + space.l;
    glyphGap = space.m;
    hintGap = space.xs;
    # A fifth of the size between dots, as Prompt.qml spaces them.
    dotSpacing = toString (points "mono" "large" * 4 / 3 * 0.2);
    aboutLeft = space.xxl;
    aboutBottom = space.xl;
    fadeFrames = toString (theme.motion.long * 50 / 1000);
  };

  art =
    pkgs.runCommand "plymouth-theme-jitsusama"
      {
        nativeBuildInputs = [
          pkgs.imagemagick
          pkgs.pango.bin
        ];
        # pango-view reads its words in the locale's encoding.
        LC_ALL = "C.UTF-8";
        FONTCONFIG_FILE = pkgs.makeFontsConf { fontDirectories = config.fonts.packages; };
      }
      ''
        export HOME=$TMPDIR
        cd $TMPDIR
        dir=$out/share/plymouth/themes/jitsusama
        mkdir -p $dir
        out=$dir
        ${drawnAt 1}
        ${drawnAt 2}
        ${lib.optionalString (picture != null) "magick ${picture} -strip $out/picture.png"}
        cp ${script} $out/jitsusama.script
        cat > $out/jitsusama.plymouth <<EOF
        [Plymouth Theme]
        Name=jitsusama
        Description=The lock screen's look, asking for the disk's PIN.
        ModuleName=script

        [script]
        ImageDir=$out
        ScriptFile=$out/jitsusama.script
        EOF
      '';

  # The face a message is set in, the one font the initrd needs.
  sansFile =
    pkgs.runCommand "plymouth-sans"
      {
        nativeBuildInputs = [ pkgs.fontconfig ];
        FONTCONFIG_FILE = pkgs.makeFontsConf { fontDirectories = config.fonts.packages; };
      }
      ''
        export HOME=$TMPDIR
        cp "$(fc-match -f '%{file}' ${lib.escapeShellArg "${sans}:style=Regular"})" $out
      '';
in
{
  config = lib.mkIf (theme != null) {
    boot.plymouth = {
      enable = true;
      theme = "jitsusama";
      themePackages = [ art ];
      font = "${sansFile}";
      # The script scales its own pictures, so Plymouth leaves the screen's
      # pixels as they are rather than doubling pictures drawn at one.
      extraConfig = ''
        DeviceScale=1
      '';
    };
  };
}
