# The checks of kernel, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  kernelSettingsHold,
  lib,
  onLinux,
  self,
  ...
}:
{
  # The panic screen wears the theme of the account the machine signs in, its
  # text on its background, and follows it when the theme changes. A machine
  # that names no account keeps the kernel's white on black. Read from the
  # kernel's settings, so no kernel is built to check it.
  the-panic-screen-wears-the-theme =
    let
      kernelOf = modules: (bareNixos modules).config.boot.kernelPackages.kernel.structuredExtraConfig;
      wearing =
        theme:
        kernelOf [
          self.nixosModules.workstation
          self.nixosModules.graphical
          self.nixosModules.kernel
          {
            jitsusama.login.account = "someone";
            home-manager.users.someone.jitsusama.theme.name = theme;
          }
        ];
      file = theme: builtins.fromTOML (builtins.readFile ../theme/${theme}.toml);
      pigment =
        theme: role:
        "0x${lib.toLower (lib.removePrefix "#" (file theme).palette.${(file theme).roles.${role}})}";
      colour =
        settings: option: (settings.${option}.content or settings.${option} or { }).freeform or null;
      follows =
        theme:
        colour (wearing theme) "DRM_PANIC_FOREGROUND_COLOR" == pigment theme "text"
        && colour (wearing theme) "DRM_PANIC_BACKGROUND_COLOR" == pigment theme "background";
      bare = kernelOf [
        self.nixosModules.workstation
        self.nixosModules.graphical
        self.nixosModules.kernel
      ];
    in
    holds "the-panic-screen-wears-the-theme"
      "The panic screen's colours aren't the account's theme, text on background."
      (follows "osaka-jade" && follows "flexoki-light" && !(bare ? DRM_PANIC_FOREGROUND_COLOR));
}
// onLinux (
  lib.mapAttrs kernelSettingsHold {
    kernel-settings-hold = self.packages.x86_64-linux.kernel;

    # Each piece of hardware builds its own kernel, with its own patches.
    kernel-settings-hold-dell-xps-14-da14260 =
      (bareNixos [ self.nixosModules.dell-xps-14-da14260 ]).config.boot.kernelPackages.kernel;
    kernel-settings-hold-microsoft-surface-laptop-4-1958 =
      (bareNixos [ self.nixosModules.microsoft-surface-laptop-4-1958 ]).config.boot.kernelPackages.kernel;

    # And with an account's theme, whose panic colours the kernel has to take.
    kernel-settings-hold-with-a-theme =
      (bareNixos [
        self.nixosModules.workstation
        self.nixosModules.graphical
        self.nixosModules.kernel
        {
          jitsusama.login.account = "someone";
          home-manager.users.someone.jitsusama.theme.name = "osaka-jade";
        }
      ]).config.boot.kernelPackages.kernel;
  }
)
