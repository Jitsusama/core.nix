# systemd-run as agents meet it, keeping their units in their own slice,
# as systemd-run.sh says why. The systemd it wraps and the cgroup file are
# given, and the real systemd-run may be, so the check can build it in
# front of a stand-in. It reads a call's arguments by
# systemd-run-options.txt, each short letter's long name and every long
# name, given it here, and is built only once that table holds to the
# options the given systemd takes.
{
  bash,
  jq,
  lib,
  runCommandLocal,
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
  holds = runCommandLocal "systemd-run-options-hold" { nativeBuildInputs = [ jq ]; } ''
    ${bash}/bin/bash ${./systemd-run-options-hold.sh} ${./systemd-run-options.txt} \
      ${systemd}/bin/systemd-run
    touch $out
  '';
in
lib.hiPrio (writeShellApplication {
  name = "systemd-run";
  # Asked for so the wrapper can't be built while its table doesn't hold.
  derivationArgs = { inherit holds; };
  text = builtins.replaceStrings (lib.attrNames filled) (lib.attrValues filled) (
    builtins.readFile ./systemd-run.sh
  );
})
