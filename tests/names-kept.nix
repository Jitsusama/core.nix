# The names of core's checks and options that say a name docs/glossary.md
# replaces and stay, for now or for good, given to tests/names.nix. A kept
# name no longer found fails, so a rename takes its entry off here.
{
  # Names whose word means something the glossary doesn't replace, often a
  # program's own word for its own thing.
  otherSenses = {
    a-quiet-file-says-nothing = "quiet is FileView printing no errors, not Do not disturb";
    backlight-passes-clippy = "the backlight program sets a screen's brightness";
    bluetooth-keeps-its-power-limits = "the limits are a radio's SAR table, not an allowance";
    "jitsusama.backlight" = "the backlight program's options, for a screen's brightness";
    "jitsusama.kernel.cpu" = "the CPU Clang's -march and -mtune name, in Clang's word";
    "jitsusama.kernel.packageOverlays" = "an overlay is Nix's, not a meter";
    "jitsusama.theme.icons" = "the icon theme is GTK's, named as it names it";
    "jitsusama.theme.palette" = "a theme's palette is its colours, not the launcher";
    "jitsusama.theme.syntax.operator" = "an operator is a code token, such as =, not a person";
    platform-profiles-reach-the-legacy-file = "the kernel's platform_profile file, by its name";
    quickshell-restarts-with-its-files = "a program restarts; the machine isn't rebooted";
    the-panel-slows-on-a-low-battery = "the panel's refresh rate slows, not lag";
    the-splash-keeps-systemd-quiet = "quiet is the kernel's quiet argument, not Do not disturb";
    vm-apps-come-first = "app is systemd's app.slice, by its name";
  };

  # Names waiting on a rename, each with the quest that owns it.
  waiting = {
    icon-marks-past-the-basic-plane = "Launcher (Y2YBTS): icon to mark";
    the-margins-glyphs-stand-level = "Launcher (Y2YBTS): glyph to mark";
    "jitsusama.theme.glyphs" = "Look and programs (QGB4ON): glyph to mark";
    palette-graphs-draw-in-braille = "Launcher (Y2YBTS): palette to launcher";
    palette-rows-keep-their-words-apart = "Launcher (Y2YBTS): palette to launcher";
    "jitsusama.quickshell.quiet.whileSharing" = "Launcher (Y2YBTS): quiet to Do not disturb";
    "jitsusama.shaper.tunnels" = "Speed (5FWIUS): tunnel to VPN";
    xdg-open-opens-each-kind-in-its-app = "Look and programs (QGB4ON): app to program";
  };
}
