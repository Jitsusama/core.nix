# The Linux console, which shows the boot's lines, the disk's PIN prompt and
# tuigreet should the greeter fail, in the theme the machine's own account
# wears, as the greeter does (../greetd/nixos.nix). The kernel takes the
# console's sixteen colours on its command line, so they hold from its first
# line. Its font is the theme's monospace face at Medium, found among the
# machine's fonts and drawn into the 16 by 32 cell the kernel picks for a
# large screen, from the glyphs in glyphs.txt (font/ says how), and loaded
# in the initrd, so the disk's prompt has it too. A machine without such an
# account, or without a theme for it, keeps the kernel's own.
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
  terminal = lib.attrByPath [ "jitsusama" "theme" "terminal" ] null home;
  face = lib.attrByPath [ "jitsusama" "theme" "font" "mono" "family" ] null home;

  # The console's order: the eight colours, then the bright eight, each as
  # six hex digits, since it has no transparency.
  eight = [
    "black"
    "red"
    "green"
    "yellow"
    "blue"
    "magenta"
    "cyan"
    "white"
  ];
  colours = map (name: lib.substring 1 6 terminal.${name}) (
    eight ++ map (name: "bright_${name}") eight
  );

  # Medium, since a console draws each pixel on or off: the regular weight's
  # stems fall on one pixel in some letters and two in others, and Medium's
  # stay at two. fontconfig answers with its nearest when the face isn't
  # there, so its answer is held to the face asked for, and the face it
  # found is written beside the font.
  font =
    pkgs.runCommandLocal "console-font"
      {
        nativeBuildInputs = [ pkgs.fontconfig ];
        FONTCONFIG_FILE = pkgs.makeFontsConf { fontDirectories = config.fonts.packages; };
      }
      ''
        export HOME=$TMPDIR
        found=$(fc-match --format '%{family}\n%{style}\n%{file}' ${lib.escapeShellArg "${face}:style=Medium"})
        family=$(echo "$found" | sed -n 1p)
        style=$(echo "$found" | sed -n 2p)
        file=$(echo "$found" | sed -n 3p)
        case ",$family," in
          *,${lib.escapeShellArg face},*) ;;
          *)
            echo "${face} isn't among the machine's fonts (fonts.packages); fontconfig offered $family" >&2
            exit 1
            ;;
        esac
        case ",$style," in
          *,Medium,*) ;;
          *)
            echo "${face} has no Medium; fontconfig offered $style" >&2
            exit 1
            ;;
        esac
        mkdir -p $out
        ${lib.getExe (pkgs.callPackage ./font/package.nix { })} "$file" ${./glyphs.txt} $out/console.psf
        printf '%s\n%s\n' "${face}" Medium > $out/face
      '';
in
{
  config = lib.mkMerge [
    (lib.mkIf (terminal != null) { console.colors = colours; })
    (lib.mkIf (face != null) {
      console.font = "${font}/console.psf";
      console.earlySetup = true;

      # udev sets the font each time a graphics driver takes the console
      # over, but once Plymouth holds the console it's busy and nothing is
      # set, so a boot whose driver comes after the splash keeps the
      # kernel's font. Each console a login prompt opens sets it again on
      # itself, which a busy console elsewhere doesn't stop.
      systemd.services."getty@".serviceConfig.ExecStartPre = [
        "-${config.systemd.package}/lib/systemd/systemd-vconsole-setup /dev/%I"
      ];
    })
  ];
}
