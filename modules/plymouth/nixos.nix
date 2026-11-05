# Plymouth, which holds the screen from the initrd until the greeter takes it,
# in the theme the machine's own account wears, as the console and greeter
# are (../console/nixos.nix, ../greetd/nixos.nix). It asks for the disk's PIN
# as the lock screen asks for a password: the lock screen's picture, the
# machine's name where the time goes, and the same panel in the same place
# (theme.script). Every word but a message the boot sends is drawn here, at
# one and two device pixels to a logical one, in the theme's faces and with
# its shadows, so the initrd carries pictures rather than fonts and what the
# screen shows is exactly what was drawn. A boot or shutdown that stalls
# says what it's waiting on (./waits). A machine without such an account, or
# without a theme for it, keeps the console.
{ wallpapers }:
{
  config,
  lib,
  options,
  pkgs,
  ...
}:
let
  inherit (import ../greetd/account.nix { inherit config lib options; }) theme;
  inherit (theme) colours;

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

  # The prompt's place, as Lock.qml and Prompt.qml build it: a field of one
  # prompt's row inside its hairline, the padlock's cell the inset rung in
  # from the frame's inside edge, what's typed as far after it as Qt puts
  # it (./glyph-width.qml), and the hint where the first letter typed will
  # be.
  inherit (theme) space;
  inherit (theme.shape) border;
  panel = {
    width = 400;
    # Prompt.qml's row, which it names by the step until it moves to the
    # rungs.
    height = space.xxxl + 2 * border;
  };
  # Room round the padlock's ink as it's drawn, more than it reaches past
  # its cell; none of it shows.
  inkRoom = space.l;
  # How far a shadow can reach, its offset and twice its blur, past which a
  # blur's tail is too faint to see.
  farthest = lib.foldl' lib.max 0;
  reach = farthest (map (shadow: shadow.offset + 2 * shadow.blur) theme.floating.shadows);
  pad = farthest (
    map (shadow: lib.max (lib.max shadow.x shadow.y) 0 + 2 * shadow.blur) theme.wallpaper.shadows
  );
  points = face: step: number theme.type.${face}.${step};
  mono = theme.font.mono.family;
  # Words to read, the ask and what the boot says, in the serif upright,
  # cut at its axes (../fonts/serif-text.nix), since Plymouth's own words
  # can't take a variable font's axes, and the pictures' words are cut alike.
  serif = theme.font.serif.text.family;
  # The voice, the field's hint, as Prompt.qml sets it: the serif's italic
  # at its axes, which pango's pictures take after an @.
  voice =
    let
      level = theme.type.levels.voice;
      family = theme.font.serif.family + lib.optionalString level.italic " Italic";
      size = toString (points "serif" level.step);
      axes = lib.concatStringsSep "," (
        lib.mapAttrsToList (tag: value: "${tag}=${toString value}") level.axes
      );
    in
    "${family} ${size} @${axes}";
  serifText = import ../fonts/serif-text.nix {
    inherit pkgs;
    inherit (theme) font;
  };
  fonts = pkgs.makeFontsConf { fontDirectories = config.fonts.packages ++ [ serifText ]; };
  # The fonts hinted as the machine hints them, for Qt, whose width for a
  # glyph moves with its hinting.
  qtFonts =
    let
      inherit (config.fonts.fontconfig) hinting;
    in
    pkgs.writeText "fonts.conf" ''
      <?xml version="1.0"?>
      <!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
      <fontconfig>
        <include>${fonts}</include>
        <match target="pattern">
          <edit name="hinting" mode="assign"><bool>${lib.boolToString hinting.enable}</bool></edit>
          <edit name="hintstyle" mode="assign"><const>hint${hinting.style}</const></edit>
        </match>
      </fontconfig>
    '';
  # How far Prompt.qml's glyph leads what's typed, as Qt measures it, in
  # the desktop's pixels, put in for the script with the pictures' measures.
  lead = ''
    lead=$(QT_QPA_PLATFORM=offscreen QT_FORCE_STDERR_LOGGING=1 FONTCONFIG_FILE=${qtFonts} \
      QML_IMPORT_PATH=${pkgs.qt6.qtdeclarative}/${pkgs.qt6.qtbase.qtQmlPrefix} \
      QT_PLUGIN_PATH=${pkgs.qt6.qtbase}/${pkgs.qt6.qtbase.qtPluginPrefix} \
      ${lib.getExe' pkgs.qt6.qtdeclarative "qml"} ${./glyph-width.qml} -- ${
        lib.escapeShellArgs [
          lockGlyph
          mono
          (toString (points "mono" "large"))
          space.m
        ]
      } 2>&1 | sed -n 's/^qml: \([0-9]*\)$/\1/p')
    [ -n "$lead" ] || { echo "Qt said nothing of how far the glyph leads"; exit 1; }
    for scale in 1 2; do
      echo 'measured["lead-'$scale'"] = '$((lead * scale))';' >> measured
    done
  '';

  # The words under the name, keyed by the picture that says them.
  under = {
    starting = "starting";
    locked = "the disk is locked";
    restarting = "restarting";
    stopping = "shutting down";
    touching = "touch the security key";
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
      hint = key: text: set (file "hint-${key}") voice colours.muted 0 text;
      words =
        out: face: size:
        set out "${face} ${toString size}";
      set = out: font: colour: margin: text: ''
        pango-view -q --no-display --dpi=${toString (96 * scale)} --margin=${at margin} \
          --antialias=gray --hinting=none --background=transparent \
          --foreground=${lib.escapeShellArg colour} \
          --font=${lib.escapeShellArg font} \
          --output=${out} --text=${lib.escapeShellArg text}
      '';
      # Where a face's capitals stand in its line, in device pixels: the top
      # of an H drawn as the pictures are, and the baseline it stands on,
      # kept for the script by name and scale, since the script places
      # words by their capitals and baselines as the lock does.
      capitals = key: font: ''
        ${set "capital.png" font "white" 0 "H"}
        ink=$(magick capital.png -format %@ info:)
        top=''${ink##*+}
        tall=''${ink#*x}
        tall=''${tall%%+*}
        echo 'measured["${key}-top-${toString scale}"] = '$top';' >> measured
        echo 'measured["${key}-baseline-${toString scale}"] = '$((top + tall))';' >> measured
      '';
      # Each shadow, the farthest first, under the words: their outline
      # blurred, darkened, coloured and moved.
      lifted = out: face: size: text: ''
        ${words "words.png" face size colours.text pad text}
        magick words.png ${
          lib.concatMapStringsSep " "
            (shadow: ''
              \( -clone 0 -channel A ${blurred shadow} \
                -evaluate multiply ${toString (number shadow.opacity)} +channel \
                -fill '${colours.shadow}' -colorize 100 -roll +${at shadow.x}+${at shadow.y} \)'')
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
                \( -size ${size} xc:none -fill '${colours.shadow}${alpha shadow.opacity}' \
                  -draw '${box 0 (shadow.offset * scale)}' ${blurred shadow} \)'')
              (lib.reverseList theme.floating.shadows)
          } \( -size ${size} xc:none -fill '${colours.deepest}${alpha theme.floating.opacity}' \
              -stroke '${edge}' -strokewidth ${at theme.shape.border} -draw '${box half 0}' \) \
            -background none -flatten ${out}
        '';
    in
    ''
      ${capitals "name" "${mono} ${toString (points "mono" "hero")}"}
      ${capitals "under" "${serif} ${toString (points "serif" "large")}"}
      ${capitals "said" "${serif} ${toString (points "serif" "body")}"}
      ${capitals "voice" voice}
      ${capitals "typed" "${mono} ${toString (points "mono" "large")}"}
      ${lifted (file "name") mono (points "mono" "hero") config.networking.hostName}
      ${lib.concatStrings (
        lib.mapAttrsToList (key: text: lifted (file key) serif (points "serif" "large") text) under
      )}
      ${lifted (file "about") mono (points "mono" "small") about}
      ${lifted (file "about-generation") mono (points "mono" "small") "${about}  ·  generation"}
      ${lib.concatMapStrings (
        digit: lifted (file "digit-${digit}") mono (points "mono" "small") digit
      ) digits}
      ${lifted (file "digits") mono (points "mono" "small") (lib.concatStrings digits)}
      ${framed (file "panel") colours.accent}
      ${framed (file "panel-urgent") colours.alert}
      # The lock's ink reaches past its cell, so it's drawn with room round
      # it and cut to the line's height; drawn without, its right side would
      # be cut away. The room is kept at its sides, so the script finds its
      # cell a room in.
      ${words "lock.png" mono large colours.machine inkRoom lockGlyph}
      magick lock.png -shave 0x${at inkRoom} +repage ${file "lock"}
      ${lib.concatStrings (lib.mapAttrsToList hint hints)}
      ${halo scale}
      ${words (file "dot") mono large colours.strong 0 "●"}
      # The caret in the accent, as tall as a line and two pixels wide.
      magick ${file "dot"} -fill '${colours.accent}' -colorize 100 \
        -channel A -fx 1 +channel -crop ${at 2}x+0+0 +repage ${file "caret"}
    '';

  # The wide shadow under a line of words drawn as the boot goes, which
  # Plymouth can't blur: a line's box blurred as the theme's widest shadow
  # blurs its words, its ends kept whole and its middle a column the script
  # stretches to the words' width. A box is denser than letters, so it's
  # drawn at half the shadow's strength.
  halo =
    scale:
    let
      at = n: toString (n * scale);
      inner = 4 * wide.blur;
      sigma = toString (wide.blur * scale / 2.0);
      file = name: "$out/${name}-${toString scale}.png";
    in
    ''
      pango-view -q --no-display --dpi=${toString (96 * scale)} --margin=0 \
        --background=transparent --font=${lib.escapeShellArg "${serif} ${toString (points "serif" "body")}"} \
        --output=line.png --text=Hg
      line=$(magick identify -format %h line.png)
      magick -size ${at (inner + 2 * pad)}x$((line + ${at (2 * pad)})) xc:none \
        -fill '${colours.shadow}' -draw "rectangle ${at pad},${at pad} ${at (pad + inner)},$((line + ${at pad} - 1))" \
        -channel A -blur 0x${sigma} -evaluate multiply ${toString (number wide.opacity / 2)} +channel \
        -roll +${at wide.x}+${at wide.y} halo.png
      magick halo.png -crop ${at (pad + inner / 2)}x+0+0 +repage ${file "halo-left"}
      magick halo.png -crop 1x+${at (pad + inner / 2)}+0 +repage ${file "halo-middle"}
      magick halo.png -crop +${at (pad + inner / 2)}+0 +repage -crop +1+0 +repage ${file "halo-right"}
    '';

  # The theme's shadows by width: the tight one drawn under live words as a
  # copy of them, the wide one as the halo.
  byBlur = lib.sort (a: b: a.blur < b.blur) theme.wallpaper.shadows;
  tight = lib.head byBlur;
  wide = lib.last byBlur;

  # The words a shutdown's last stage is said in. Whatever bounds the
  # shutdown sends them as keywords, so they're said here, in one voice.
  stages = {
    forced = "ending what wouldn't stop";
    leftovers = "waiting on the last programs";
  };

  # What's booting, small at the bottom left.
  about =
    let
      inherit (config.system.nixos) release;
      inherit (config.boot.kernelPackages.kernel) version;
    in
    "NixOS ${release}  ·  Linux ${version}";
  # The generation after it, which only the boot knows: the line with the
  # word that comes before the number, and each digit, which the script puts
  # together as it's told the number. The face is monospaced, so a digit
  # sits a digit's width from the one before, as one line would have it.
  digits = map toString (lib.range 0 9);
  lockGlyph = theme.marks.secret.glyph;

  script = pkgs.replaceVars ./theme.script {
    inherit pad reach;
    deepest = channels colours.deepest;
    textColour = channels colours.text;
    serifFamily = serif;
    serifBody = toString (points "serif" "body");
    shadowColour = channels colours.shadow;
    tightX = tight.x;
    tightY = tight.y;
    tightOpacity = toString (number tight.opacity);
    haloInner = 4 * wide.blur;
    stages = lib.concatStrings (
      lib.mapAttrsToList (key: text: ''
        stages["${key}"] = "${text}";
      '') stages
    );
    # Measured as the pictures are drawn, and put in as they are.
    measured = null;
    inherit (space) beat;
    inherit (space.rung) apart screen;
    panelWidth = panel.width;
    panelHeight = panel.height;
    # The padlock's cell, from the frame's outside edge.
    markAt = border + space.rung.inset;
    inherit inkRoom;
    # A fifth of the size between dots, as Prompt.qml spaces them.
    dotSpacing = toString (points "mono" "large" * 4 / 3 * 0.2);
    digitImages = lib.concatMapStrings (digit: ''
      global.about.digits["${digit}"] = drawn("digit-${digit}");
    '') digits;
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
        FONTCONFIG_FILE = fonts;
      }
      ''
        export HOME=$TMPDIR
        cd $TMPDIR
        dir=$out/share/plymouth/themes/jitsusama
        mkdir -p $dir
        out=$dir
        ${drawnAt 1}
        ${drawnAt 2}
        ${lead}
        ${lib.optionalString (picture != null) "magick ${picture} -strip $out/picture.png"}
        substitute ${script} $out/jitsusama.script --replace-fail @measured@ "$(cat measured)"
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

  waits = pkgs.callPackage ./waits/package.nix { };

  # The shutdown's screens, which systemd's last stage would otherwise end
  # with its first kill, the moment it starts waiting on what's left: the
  # screen froze at a hung restart with nothing more to say.
  shutdowns = [
    "plymouth-halt"
    "plymouth-kexec"
    "plymouth-poweroff"
    "plymouth-reboot"
  ];

  # Tells the splash the generation the boot loader started, from the entry
  # systemd-boot says it chose: nixos-generation-192-<hash>.efi as
  # lanzaboote names it. The variable is four bytes of attributes, then the
  # name in UTF-16. NixOS's own systemd-boot entries are named by a hash of
  # what they hold, so they and a boot systemd-boot didn't start say
  # nothing, and the splash keeps its line without a generation.
  tellsGeneration = plymouth: ''
    entry=$(tail -c +5 /sys/firmware/efi/efivars/LoaderEntrySelected-4a67b082-0a4c-41cf-b6c7-440b29bb8c4f 2>/dev/null | tr -d '\0')
    rest=''${entry#nixos-generation-}
    number=''${rest%%[!0-9]*}
    if [ "$rest" != "$entry" ] && [ -n "$number" ]; then
      ${plymouth} update --status="generation:$number"
    fi
  '';
in
{
  config = lib.mkIf (theme != null) {
    # greetd starts as Plymouth lets go of the screen, below, rather than
    # once it's quit.
    services.greetd.greeterManagesPlymouth = true;
    # Counted from greetd's start, since niri opens the GPU about a second
    # after; ten seconds leaves a slow boot room. Should niri not have
    # opened it by then, the console shows until it does, as it did before,
    # and a boot whose greetd fails still quits the splash. Timers start
    # before basic.target, which greetd waits on, so this one leaves its
    # usual order out.
    systemd.timers.plymouth-quit = {
      wantedBy = [ "multi-user.target" ];
      after = [ "greetd.service" ];
      before = [ "shutdown.target" ];
      conflicts = [ "shutdown.target" ];
      unitConfig.DefaultDependencies = false;
      timerConfig.OnActiveSec = "10s";
    };
    boot.plymouth = {
      enable = true;
      # plymouth quits with the splash kept (plymouth-quit below), which
      # frees it while shown and left its frame timer to fire on the freed
      # splash: plymouthd crashed so in a starved VM's boot. The patch
      # (frame-timer.patch) stops the timer as the splash is freed. And a
      # status sent before the splash is made, as the generation is when
      # the graphics card comes late, reached nothing and was lost, so the
      # second (statuses-kept.patch) keeps each for a splash made later.
      package = pkgs.plymouth.overrideAttrs (old: {
        patches = (old.patches or [ ]) ++ [
          ./frame-timer.patch
          ./statuses-kept.patch
        ];
      });
      theme = "jitsusama";
      themePackages = [ art ];
      # The face a message is set in, the one font the initrd needs.
      font = serifText.file;
      # The script scales its own pictures, so Plymouth leaves the screen's
      # pixels as they are rather than doubling pictures drawn at one.
      extraConfig = ''
        DeviceScale=1
      '';
    };
    # systemd writes a line for each unit to the console unless told
    # otherwise, and the first woke the console over the firmware's logo,
    # so its lines showed until the GPU's own screen came for Plymouth, 1.8 s
    # later, and again as a shutdown began. A failure is still written.
    boot.kernelParams = [ "systemd.show_status=error" ];

    # As soon as the splash is up, so the generation shows before the disk
    # asks for its PIN.
    boot.initrd.systemd.services.plymouth-generation = {
      description = "Tell Plymouth the generation the boot loader started";
      wantedBy = [ "sysinit.target" ];
      after = [ "plymouth-start.service" ];
      unitConfig.DefaultDependencies = false;
      serviceConfig.Type = "oneshot";
      script = tellsGeneration "plymouth";
    };

    systemd.services =
      lib.genAttrs shutdowns (_: {
        unitConfig.SurviveFinalKillSignal = true;
      })
      // {
        # Plymouth's last frame stays on the screen until niri draws over
        # it. Quit before niri opened the GPU, plymouthd closed the last
        # hold on it, and the kernel put its console's framebuffer back for
        # the seconds between (drm_lastclose): on a laptop docked with its
        # lid shut, a console sized for the shut panel in the monitor's top
        # corner. So, as GDM does, Plymouth lets go of the screen as greetd
        # starts, keeping the GPU open (`plymouth deactivate` drops only its
        # hold on the display), and quits once niri has had time to open it.
        # With the hold dropped, a console set to text draws it all the same,
        # which greetd did until its patch (modules/greetd) left it in
        # graphics.
        # The guide quits it sooner when it has something to show.
        plymouth-quit = {
          wantedBy = lib.mkForce [ ];
          serviceConfig.ExecStart = [
            ""
            "-${config.boot.plymouth.package}/bin/plymouth quit --retain-splash"
          ];
        };
        # Nothing waits on Plymouth's quitting now, which comes after the
        # boot's other jobs.
        plymouth-quit-wait.wantedBy = lib.mkForce [ ];
        # The client waits on plymouthd without a limit of its own, so a
        # hung plymouthd would hold the sign-in for greetd's whole start
        # timeout; five seconds leaves one request plenty of room.
        greetd = {
          serviceConfig.ExecStartPre = [
            (lib.concatStringsSep " " [
              "-${lib.getExe' pkgs.coreutils "timeout"}"
              "5"
              "${config.boot.plymouth.package}/bin/plymouth"
              "deactivate"
            ])
          ];
        };

        # From the switch to the disk's system until Plymouth quits, and
        # from a shutdown's start to its end. It needs nothing started
        # first, and stays out of the way of every other unit's order.
        # A shutdown's splash is a new one, so it's told the generation again.
        plymouth-generation = {
          description = "Tell Plymouth the generation the boot loader started";
          wantedBy = [
            "halt.target"
            "kexec.target"
            "poweroff.target"
            "reboot.target"
          ];
          after = map (unit: "${unit}.service") shutdowns;
          unitConfig.DefaultDependencies = false;
          serviceConfig.Type = "oneshot";
          script = tellsGeneration (lib.getExe' config.boot.plymouth.package "plymouth");
        };

        plymouth-waits = {
          description = "Tell Plymouth what the boot or shutdown waits on";
          wantedBy = [
            "sysinit.target"
            "halt.target"
            "kexec.target"
            "poweroff.target"
            "reboot.target"
          ];
          unitConfig.DefaultDependencies = false;
          serviceConfig.ExecStart = lib.getExe waits;
        };
      };
  };
}
