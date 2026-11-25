# systemd-run as agents meet it, keeping their units in their own slice,
# as systemd-run.sh says why. The systemd it wraps and the cgroup file are
# given, and the real systemd-run may be, so the check can build it in
# front of a stand-in. It reads a call's arguments by
# systemd-run-options.txt, each short letter's long name and every long
# name, given it here, and is built only once that table holds to the
# options the given systemd takes. A machine built elsewhere, for a
# platform its builder can't run, has the table held to the builder's own
# systemd instead where that's built from the same source, patches and
# flags, since run.c lists the same options on every platform. Where it's neither,
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
  # The build's flags that name no platform: a store path, such as
  # util-linux's mount, is the platform's own build, so it differs between
  # a machine's systemd and a builder's built alike, while a feature turned
  # off, through the flags or the override argument that sets one, doesn't.
  alike = drv: lib.filter (flag: !builtins.hasContext flag) (map toString (drv.mesonFlags or [ ]));
  builders = buildPackages.systemd;
  canHave = lib.meta.availableOn stdenv.buildPlatform builders;
  # Source, patches and those flags are compared, and nothing else: not
  # postPatch, since nixpkgs' names each platform's own compiler and
  # readelf, so it differs between a machine's systemd and a builder's built
  # alike; nor the other phases or the compiler's environment; nor a flag
  # that names a store path, which alike drops whole. A change to run.c's
  # options through any of them is the hold's blind spot.
  differs =
    lib.optional ((builders.src or null) != (systemd.src or null)) "source"
    ++ lib.optional ((builders.patches or [ ]) != (systemd.patches or [ ])) "patches"
    ++ lib.optional (alike builders != alike systemd) "flags";
  asked =
    if stdenv.buildPlatform.canExecute stdenv.hostPlatform then
      systemd
    else if canHave && differs == [ ] then
      builders
    else
      null;
  named = if systemd ? version then "systemd ${systemd.version}" else "one of unknown version";
  # The first few flags only one side has, so a reader sees what to drop.
  ownFlags = lib.take 3 (lib.subtractLists (alike builders) (alike systemd));
  missing = lib.take 3 (lib.subtractLists (alike systemd) (alike builders));
  flagsSaid = lib.concatStringsSep ", " (ownFlags ++ map (flag: "no ${flag}") missing);
  whatDiffers =
    lib.concatStringsSep ", " differs
    + lib.optionalString (lib.elem "flags" differs && flagsSaid != "") " (${flagsSaid})";
  why =
    if canHave then
      "has none built alike, since the machine's differs in its ${whatDiffers}; build it on "
      + "${stdenv.hostPlatform.system}, or drop what differs from nixpkgs' systemd, to hold it"
    else
      "can't build systemd at all; build it on ${stdenv.hostPlatform.system} to hold it";
  unheld =
    "systemd-run's wrapper isn't held to ${named}, so a call it misreads goes unseen: "
    + "${stdenv.buildPlatform.system}, which builds it, can't run "
    + "${stdenv.hostPlatform.system}'s systemd and ${why}";
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
  # Only a wrapper nothing holds says so, so a check of what it says can't
  # pass on one that's held.
  passthru = {
    held = asked != null;
  }
  // (if asked != null then { inherit holds; } else { inherit unheld; });
  text = builtins.replaceStrings (lib.attrNames filled) (lib.attrValues filled) (
    builtins.readFile ./systemd-run.sh
  );
})
