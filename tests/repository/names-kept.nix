# The names of core's checks and options that say a name docs/glossary.md
# replaces and stay, for now or for good, given to
# tests/repository/names.nix. A kept name no longer found fails, so a rename
# takes its entry off here.
{
  # Names whose word means something the glossary doesn't replace, often a
  # program's own word for its own thing.
  otherSenses = {
    a-quiet-file-says-nothing = "quiet is FileView printing no errors, not Do not disturb";
    backlight-passes-clippy = "the backlight program sets a screen's brightness";
    bluetooth-keeps-its-power-limits = "the limits are a radio's SAR table, not an allowance";
    chrome-recent-sites-reads-a-history-mid-write = "Chrome's History database, by its name";
    clipboard-history-passes-clippy = "the clipboard-history program, by its name";
    "jitsusama.backlight" = "the backlight program's options, for a screen's brightness";
    "jitsusama.kernel.cpu" = "the CPU Clang's -march and -mtune name, in Clang's word";
    "jitsusama.kernel.packageOverlays" = "an overlay is Nix's, not a meter";
    "jitsusama.keys.google-drive" = "Google Drive's keys, as Google names its file store, not a disk";
    "jitsusama.nix-offload.lines" =
      "a line is a builder's entry in Nix's machines files, not a connection";
    "jitsusama.pi.hosts" =
      "a pi host is the harness's process holding many pi conversations, not a machine";
    "jitsusama.theme.icons" = "the icon theme is GTK's, named as it names it";
    "jitsusama.theme.palette" = "a theme's palette is its colours, not the launcher";
    "jitsusama.theme.syntax.operator" = "an operator is a code token, such as =, not a person";
    "jitsusama.update-to-main.batches" =
      "a batch is one build's share of a switch's pieces, not a fleet of agents";
    "jitsusama.update-to-main.holdAccountRestarts" =
      "an account's daemons restart as systemd restarts them; the machine isn't rebooted";
    "jitsusama.update-to-main.holdRestarts" =
      "a daemon restarts as systemd restarts it; the machine isn't rebooted";
    linux-builder-lets-only-those-named-restart-it = "a VM and a service restart; the Mac isn't rebooted";
    nix-offload-lines-reach-root-alone = "a line is a builder's entry in Nix's machines files, not a connection";
    pi-hosts-refuse-an-unproven-runtime = "a pi host is the harness's process holding many pi conversations, not a machine";
    pi-hosts-start-with-the-session = "a pi host is the harness's process holding many pi conversations, not a machine";
    platform-profiles-reach-the-legacy-file = "the kernel's platform_profile file, by its name";
    quickshell-restarts-with-its-files = "a program restarts; the machine isn't rebooted";
    the-panel-slows-on-a-low-battery = "the panel's refresh rate slows, not lag";
    the-splash-keeps-systemd-quiet = "quiet is the kernel's quiet argument, not Do not disturb";
    vm-apps-come-first = "app is systemd's app.slice, by its name";
  };

  # Names waiting on a rename, each with the quest that owns it.
  waiting = {
  };
}
