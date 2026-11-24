# The checks of name-lookups, which checks.nix gathers with every other
# folder's; each says what it proves.
{
  account,
  lib,
  macWorkstation,
  onLinux,
  pkgs,
  workstation,
  ...
}:
# What follows runs the round over stand-ins on Linux, which has no
# dscacheutil; the Mac's account only gives it its names.
onLinux {
  # A Mac's account times its names each minute through the system's
  # resolver, and a round says a failed lookup at once, a slow one from its
  # second round running, a busy mDNSResponder from its fifth, and each well
  # again once (tests/lookups-say-a-name-that-fails-or-takes-long.sh).
  # Linux's account runs no such agent, since its resolver's journal says as
  # much already.
  lookups-say-a-name-that-fails-or-takes-long =
    let
      agent = machine: (account machine).launchd.agents.name-lookups or { };
      mac = agent macWorkstation;
      arguments = mac.config.ProgramArguments or [ ];
    in
    assert lib.assertMsg (
      mac.enable or false && lib.elem "${./lookups.sh}" arguments
    ) "A Mac's account doesn't time its name lookups.";
    assert lib.assertMsg (lib.elem "github.com" arguments)
      "A Mac's account looks up none of the names it can't work without.";
    assert lib.assertMsg (
      (agent workstation).enable or false == false
    ) "A Linux account runs the Mac's name lookups.";
    pkgs.runCommandLocal "lookups-say-a-name-that-fails-or-takes-long" { } ''
      ${lib.getExe pkgs.bash} ${./tests/lookups-say-a-name-that-fails-or-takes-long.sh} \
        ${./lookups.sh}
      touch $out
    '';
}
