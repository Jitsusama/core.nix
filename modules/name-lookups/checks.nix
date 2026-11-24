# The checks of name-lookups, which checks.nix gathers with every other
# folder's; each says what it proves.
{
  account,
  lib,
  macWorkstation,
  onLinux,
  pkgs,
  ...
}:
# What follows runs the round over stand-ins on Linux, which has no
# dscacheutil; the Mac's account only gives it its names.
onLinux {
  # A Mac's account times its names each minute through the system's
  # resolver, given in the order the round reads them, at Standard, and a
  # round says a failed or slow lookup from its second round running, either
  # from its third, a busy mDNSResponder from its fifth, and each well again
  # once
  # (tests/lookups-say-a-name-that-fails-or-takes-long.sh).
  lookups-say-a-name-that-fails-or-takes-long =
    let
      mac = (account macWorkstation).launchd.agents.name-lookups or { };
      arguments = mac.config.ProgramArguments or [ ];
    in
    assert lib.assertMsg (
      mac.enable or false && lib.take 1 (lib.drop 1 arguments) == [ "${./lookups.sh}" ]
    ) "A Mac's account doesn't time its name lookups.";
    assert lib.assertMsg (
      lib.drop 3 arguments == [
        "500"
        "github.com"
        ""
      ]
    ) "A Mac's account doesn't give the slow mark, names and servers in order.";
    assert lib.assertMsg (
      mac.config.ProcessType or null == "Standard"
    ) "A Mac's account doesn't run its lookups at Standard, so they'd time a wait for a core.";
    pkgs.runCommandLocal "lookups-say-a-name-that-fails-or-takes-long" { } ''
      ${lib.getExe pkgs.bash} ${./tests/lookups-say-a-name-that-fails-or-takes-long.sh} \
        ${./lookups.sh}
      touch $out
    '';
}
