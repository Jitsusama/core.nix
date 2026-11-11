# The checks of the launcher's Keep Awake, which the folder's checks.nix gathers
# with every other place's; its opening comment says why each place keeps its
# own.
{
  account,
  bareNixos,
  holds,
  lib,
  self,
  ...
}:
{
  # Keeping awake holds the lid only where a shut lid would suspend, on
  # battery or on the charger: a machine with the lid module, whose lid
  # suspends on battery, keeps the lid holder and its shell says so, and
  # one whose lid is ignored everywhere has neither.
  keeping-awake-holds-the-lid-only-where-it-sleeps =
    let
      lidded = bareNixos [
        self.nixosModules.graphical
        self.nixosModules.lid
      ];
      ignored = bareNixos [
        self.nixosModules.graphical
        {
          services.logind.settings.Login = {
            HandleLidSwitch = "ignore";
            HandleLidSwitchExternalPower = "ignore";
          };
        }
      ];
      holder = machine: (account machine).systemd.user.services ? keep-awake-lid;
      says =
        machine:
        lib.hasInfix "lidSleeps: true" (account machine).xdg.configFile."quickshell/Options.qml".text;
    in
    holds "keeping-awake-holds-the-lid-only-where-it-sleeps"
      "the lid's holder and the shell's word should both follow whether a shut lid suspends, on battery or on the charger"
      (holder lidded && says lidded && !holder ignored && !says ignored);
}
