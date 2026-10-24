# The checks of yubikey, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  holds,
  lib,
  macWorkstation,
  workstation,
  ...
}:
{
  # The YubiKey gets Joel's keys back on any machine he writes code on, so both
  # workstation roles bring its tools, a Mac as much as Linux.
  yubikey-on-every-workstation =
    let
      hasYkman =
        machine:
        lib.any (
          package: lib.getName package == "yubikey-manager"
        ) machine.config.environment.systemPackages;
    in
    holds "yubikey-on-every-workstation" "A workstation lacks the YubiKey's tools." (
      hasYkman workstation && hasYkman macWorkstation
    );
}
