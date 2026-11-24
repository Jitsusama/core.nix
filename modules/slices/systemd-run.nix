# systemd-run as agents meet it, keeping their units in their own slice,
# as systemd-run.sh says why. The systemd it wraps and the cgroup file are
# given, and the real systemd-run may be, so the check can build it in
# front of a stand-in. It reads a call's arguments by
# systemd-run-options.txt, each short letter's long name and every long
# name, given it here, and is built only once that table holds to the
# options the given systemd takes. A machine built elsewhere, for a
# platform its builder can't run, has the table held to the builder's own
# systemd instead where that's built from the same source and patches,
# since run.c lists the same options on every platform. Where it's neither,
# or the builder can't have systemd at all, the wrapper says it isn't held
# (held, and unheld for the warning a machine raises), so a machine's
# evaluation warns. Where the hold runs, any difference fails the machine's
# build, a harmless one too, rather than guessing which a wrapper would
# misread; the way on is a change to the table in core, as the hold's
# refusal says.
{
  buildPackages,
  jq,
  lib,
  runCommandLocal,
  stdenv,
  systemd,
  writeShellApplication,
  real ? "${systemd}/bin/systemd-run",
  cgroup ? "/proc/self/cgroup",
}:
let
  lines = lib.splitString "\n" (builtins.readFile ./systemd-run-options.txt);
  options = map (lib.splitString " ") (
    lib.filter (line: line != "" && !lib.hasPrefix "#" line) lines
  );
  letter = option: builtins.elemAt option 0;
  long = option: builtins.elemAt option 1;
  shorts = lib.concatMapStringsSep " " (option: ''"${letter option}:${long option}"'') (
    lib.filter (option: letter option != "-") options
  );
  longs = lib.concatMapStringsSep " " (option: ''"${long option}"'') options;
  filled = {
    "@real@" = real;
    "@cgroup@" = cgroup;
    "@shorts@" = shorts;
    "@longs@" = longs;
  };
  asked =
    if stdenv.buildPlatform.canExecute stdenv.hostPlatform then
      systemd
    # Source and patches are compared, not postPatch or the build's flags:
    # nixpkgs' postPatch names each platform's own compiler and readelf, so
    # it differs between a machine's systemd and a builder's built alike. A
    # change to run.c's options through them is the hold's blind spot.
    else if
      lib.meta.availableOn stdenv.buildPlatform buildPackages.systemd
      && (buildPackages.systemd.src or null) == (systemd.src or null)
      && (buildPackages.systemd.patches or [ ]) == (systemd.patches or [ ])
    then
      buildPackages.systemd
    else
      null;
  named = if systemd ? version then "systemd ${systemd.version}" else "one of unknown version";
  unheld =
    "systemd-run's wrapper isn't held to ${named}: the builder can't run "
    + "it and has no systemd built as it is, so a call it misreads goes unseen; "
    + "build it where its systemd runs, or on a builder with one built alike, to hold it";
  # Only a wrapper something holds has a hold, so reading one never throws.
  holds =
    if asked == null then
      null
    else
      runCommandLocal "systemd-run-options-hold" { nativeBuildInputs = [ jq ]; } ''
        ${buildPackages.bash}/bin/bash ${./systemd-run-options-hold.sh} \
          ${./systemd-run-options.txt} ${asked}/bin/systemd-run
        touch $out
      '';
in
lib.hiPrio (writeShellApplication {
  name = "systemd-run";
  # Asked for so the wrapper can't be built while its table doesn't hold. The
  # hold runs where the machine is built, so its bash is the builder's.
  derivationArgs = lib.optionalAttrs (asked != null) { inherit holds; };
  passthru = {
    held = asked != null;
    inherit unheld;
  }
  // lib.optionalAttrs (asked != null) { inherit holds; };
  text = builtins.replaceStrings (lib.attrNames filled) (lib.attrValues filled) (
    builtins.readFile ./systemd-run.sh
  );
})
