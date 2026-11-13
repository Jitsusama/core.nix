# The checks of quickshell, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves. Each place the
# launcher names, and each surface beside it, keeps its checks in a file of
# its own under checks/, so two places' changes never share a file: a check
# goes with the one place whose QML it copies, and one copying several
# places', or the launcher's own machinery, goes in checks/core.nix. What
# more than one uses is in checks/helpers.nix, given to each beside the
# folder's own arguments. A new place adds its file to the list below.
args@{ lib, onLinux, ... }:
let
  given = args // import ./checks/helpers.nix args;
  files = [
    ./checks/accessories.nix
    ./checks/agents.nix
    ./checks/askpass.nix
    ./checks/asks.nix
    ./checks/capture.nix
    ./checks/clipboard.nix
    ./checks/command-line.nix
    ./checks/core.nix
    ./checks/displays.nix
    ./checks/documents.nix
    ./checks/emoji.nix
    ./checks/files.nix
    ./checks/heat.nix
    ./checks/home.nix
    ./checks/keep-awake.nix
    ./checks/location.nix
    ./checks/looks.nix
    ./checks/machine.nix
    ./checks/margin.nix
    ./checks/networks.nix
    ./checks/notices.nix
    ./checks/passwords.nix
    ./checks/people.nix
    ./checks/power.nix
    ./checks/programs.nix
    ./checks/quests.nix
    ./checks/ready-rows.nix
    ./checks/selection.nix
    ./checks/sessions.nix
    ./checks/shell.nix
    ./checks/shortcuts.nix
    ./checks/tabs.nix
    ./checks/sound.nix
    ./checks/spend.nix
    ./checks/terminal.nix
    ./checks/timers.nix
    ./checks/turned-on.nix
    ./checks/windows.nix
    ./checks/words.nix
  ];
  places = map (file: {
    inherit file;
    checks = import file given;
  }) files;
  # A name two places give stops the evaluation rather than one check
  # hiding the other.
  gathered = lib.foldl' (
    gathered: these:
    let
      clashes = lib.intersectLists (lib.attrNames gathered) (lib.attrNames these);
    in
    if clashes == [ ] then
      gathered // these
    else
      throw "More than one of quickshell's places gives ${lib.concatStringsSep ", " clashes}"
  ) { } (map (place: place.checks) places);
  # The place's file giving each check, from the repository's root, which
  # checks.nix says for it rather than this one, so a change to one place's
  # file reaches only its checks and those of a file naming it. Only the
  # names are read, so no check is evaluated to say it.
  checkFiles = lib.foldl' (
    giving: place:
    let
      file = lib.removePrefix "./" (lib.path.removePrefix ../.. place.file);
    in
    giving // lib.mapAttrs (_: _: file) place.checks
  ) { } places;
in
# The shell runs only on Linux, so a Mac has none of its checks: most build
# Quickshell or a program only Linux has, and the rest read a Linux machine,
# which Linux's own checks already do.
onLinux gathered // { inherit checkFiles; }
