# The Linux console, which shows the boot's lines, the disk's PIN prompt and
# tuigreet should the greeter fail, in the theme the machine's own account
# wears, as the greeter does (../greetd/nixos.nix). The kernel takes the
# console's sixteen colours on its command line, so they hold from its first
# line. Its font is the theme's monospace face at Medium, found among the
# machine's fonts and drawn into the 16 by 32 cell the kernel picks for a
# large screen, from the glyphs in glyphs.txt (font/ says how), and loaded
# in the initrd, so the disk's prompt has it too. Above each login prompt, a
# console greets the account as the greeter does and says the way back to
# the desktop, which kmscon's consoles read too (../kmscon/nixos.nix). A
# failed boot's shell says what stopped it first (stopped.sh). A machine
# without such an account, or
# without a theme for it, keeps the kernel's own.
{
  config,
  lib,
  options,
  pkgs,
  ...
}:
let
  inherit (import ../greetd/account.nix { inherit config lib options; }) account theme;
  terminal = lib.attrByPath [ "terminal" ] null theme;
  face = lib.attrByPath [ "font" "mono" "family" ] null theme;

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

  # A role's colour as the console's SGR number: the one of its sixteen
  # holding the role's colour, or its own foreground when none does.
  slot =
    role:
    let
      names = eight ++ map (name: "bright_${name}") eight;
      at = lib.lists.findFirstIndex (
        name: lib.toLower terminal.${name} == lib.toLower theme.colours.${role}
      ) null names;
    in
    if at == null then "39" else toString (if at < 8 then 30 + at else 82 + at);

  # A piece of text in a role's colour.
  escape = builtins.fromJSON ''"\u001b"'';
  paint = role: words: "${escape}[${slot role}m${words}${escape}[0m";

  # The account by the first word of its description, as the greeter calls
  # it (../greetd/nixos.nix).
  called =
    let
      description = config.users.users.${account}.description or "";
    in
    if description == "" then account else builtins.head (lib.splitString " " description);

  # What a console says above its login prompt, in place of NixOS's banner:
  # the greeter's greeting, and the keys back to it, drawn as the failed
  # boot's keys are. Nothing in it names the console, which a console
  # kmscon draws would name by its pseudo-terminal.
  issue = lib.concatLines [
    "${paint "text" "Hello,"} ${paint "strong" called}"
    "${paint "strong" "Ctrl+Alt+F1"} ${paint "muted" "goes back to the desktop."}"
    ""
  ];

  # Says what stopped the boot (stopped.sh), in the theme's colours.
  stopped = pkgs.writeShellApplication {
    name = "boot-stopped";
    runtimeInputs = [
      config.systemd.package
      pkgs.coreutils
    ];
    text = lib.concatLines [
      "alert=${slot "alert"} strong=${slot "strong"} text=${slot "text"} muted=${slot "muted"}"
      "mark=${lib.escapeShellArg theme.marks.failed.console}"
      (builtins.readFile ./stopped.sh)
    ];
  };

  # What sets a console's font and keymap, given the console.
  setup = "${config.systemd.package}/lib/systemd/systemd-vconsole-setup";

  # Medium, since a console draws each pixel on or off: the regular weight's
  # stems fall on one pixel in some letters and two in others, and Medium's
  # stay at two. fontconfig answers with its nearest when the face isn't
  # there, so its answer is held to the face asked for, and the face it
  # found is written beside the font (font.sh).
  font = pkgs.runCommandLocal "console-font" {
    nativeBuildInputs = [ pkgs.fontconfig ];
    FONTCONFIG_FILE = pkgs.makeFontsConf { fontDirectories = config.fonts.packages; };
    inherit face;
    glyphs = ./glyphs.txt;
    convert = lib.getExe (pkgs.callPackage ./font/package.nix { });
  } (builtins.readFile ./font.sh);
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
      # itself, which a busy console elsewhere doesn't stop, and a failed
      # boot's emergency shell sets it on the console in front once Plymouth
      # has let go of it: after, since Plymouth's module empties its list
      # first.
      systemd.services = {
        "getty@".serviceConfig.ExecStartPre = [ "-${setup} /dev/%I" ];
        emergency.serviceConfig.ExecStartPre = lib.mkAfter [ "-${setup} /dev/tty0" ];
      };
    })
    # A failed boot's emergency shell says what stopped it, after the face
    # is set, which comes after Plymouth lets go; the initrd's says so too,
    # since it carries the shell, coreutils and systemd the summary runs on.
    (lib.mkIf (terminal != null) { environment.etc.issue.text = issue; })
    (lib.mkIf (terminal != null) {
      systemd.services.emergency.serviceConfig.ExecStartPre = lib.mkOrder 1600 [
        "-${lib.getExe stopped}"
      ];
      boot.initrd.systemd = lib.mkIf config.boot.initrd.systemd.enable {
        storePaths = [ stopped ];
        services.emergency.serviceConfig.ExecStartPre = lib.mkOrder 1600 [
          "-${lib.getExe stopped}"
        ];
      };
    })
  ];
}
