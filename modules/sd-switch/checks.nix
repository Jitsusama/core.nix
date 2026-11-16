# The checks of sd-switch, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  bareNixos,
  holds,
  lib,
  self,
  ...
}:
let
  machine = bareNixos [ self.nixosModules.base ];
  inherit (machine.pkgs) sd-switch;
  patched = lib.any (
    patch: baseNameOf (toString patch) == "sd-switch-reads-as-it-sends.patch"
  ) sd-switch.patches;
  # Read as plain text, since a search may not carry the store paths it names.
  plain = builtins.unsafeDiscardStringContext;
  activation = plain (account machine).home.activation.reloadSystemd.data;
in
{
  # Every NixOS account switches its units with the sd-switch that reads each
  # job's end as it comes, as the base role brings it, so a switch of more
  # than 64 jobs doesn't hang.
  an-account-switch-hears-each-job-end =
    holds "an-account-switch-hears-each-job-end"
      "An account's units switch with an sd-switch that reads no job's end until it has sent every job."
      (patched && lib.hasInfix (plain (lib.getExe sd-switch)) activation);
  # The patched sd-switch builds and passes its own tests, which run as it builds.
  sd-switch-passes-its-tests = sd-switch;
}
