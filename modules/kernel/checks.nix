# The checks of kernel, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  bareNixos,
  holds,
  lib,
  nixpkgs,
  onLinux,
  pkgs,
  self,
  ...
}:
let
  # The line a kernel setting leaves in the finished configuration.
  kernelConfigLine =
    option: setting:
    if setting ? freeform then
      "CONFIG_${option}=${setting.freeform}"
    else if setting.tristate == "n" then
      "# CONFIG_${option} is not set"
    else
      "CONFIG_${option}=${setting.tristate}";

  # The kernel's patches apply, its configuration resolves, and every setting
  # in settings.nix is in the result: Kconfig quietly drops a setting whose
  # dependencies aren't met. An optional setting also holds when the kernel
  # doesn't offer its option at all. Only the configuration is built, in
  # minutes; the kernel itself takes far longer than a check should
  # (tests/kernel-settings-hold.sh).
  kernelSettingsHold =
    name: kernel:
    let
      # The kernel's own, as package.nix gave them, so a machine's theme
      # colours are held as well as the settings every kernel shares.
      settings = lib.mapAttrs (_option: setting: setting.content or setting) kernel.structuredExtraConfig;
      # A setting's line in the configuration, its option, and whether it's
      # optional, by tabs.
      row =
        option: setting:
        lib.concatStringsSep "\t" [
          (kernelConfigLine option setting)
          option
          (if setting.optional or false then "optional" else "required")
        ];
      rows = pkgs.writeText "${name}-settings" (lib.concatLines (lib.mapAttrsToList row settings));
    in
    pkgs.runCommandLocal name { } ''
      ${lib.getExe pkgs.bash} ${./tests/kernel-settings-hold.sh} ${kernel.configfile} ${rows}
      touch $out
    '';

  # A few files of a kernel's source, with its own patches applied to them,
  # as the machine builds it (tests/patched-kernel-source.sh).
  kernelSource =
    kernel: files:
    pkgs.runCommandLocal "patched-source" { nativeBuildInputs = [ pkgs.patchutils ]; } ''
      ${lib.getExe pkgs.bash} ${./tests/patched-kernel-source.sh} \
        ${./tests/kernel-source-files.sh} ${kernel.src} ${
          pkgs.writeText "patches" (
            lib.concatMapStrings (patch: "${patch.patch}\n") (
              lib.filter (patch: patch.patch != null) kernel.kernelPatches
            )
          )
        } $out ${lib.escapeShellArgs files}
    '';
in
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
        "0x${lib.toLower (lib.removePrefix "#" (file theme).palette.${(file theme).colours.${role}})}";
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
// onLinux {
  # The checks reading a kernel's source take only a few of its files. A
  # patch creating one a newer kernel has too fails the build, and failed
  # the check only as patch's own refusal, which names no patch. So taking
  # them names the patch, refuses a file no patch creates rather than let a
  # check read nothing, and lets a file a patch creates be missing, or be
  # there where a patch before deletes it (tests/kernel-source-files.sh). A
  # tiny source, a patch creating one file and one deleting it stand in for
  # a kernel's, in a folder and in a tarball alike.
  kernel-source-files-are-taken-as-built =
    pkgs.runCommandLocal "kernel-source-files-are-taken-as-built"
      { nativeBuildInputs = [ pkgs.patchutils ]; }
      ''
        ${lib.getExe pkgs.bash} ${./tests/kernel-source-files-cases.sh} \
          ${./tests/kernel-source-files.sh}
        touch $out
      '';

  # A join through wpa_supplicant recorded the network's name only after
  # finding the access point's country, and one that names none ended the
  # join's last step first, so networkd couldn't match the network's file.
  # Core's kernel carried the fix for each machine that builds it until
  # 7.3.6 took it, so the kernel it builds must still name the network
  # first (tests/ssid-before-country.sh). This reads core's kernel with its
  # own patches; a machine whose patches change sme.c reads its own.
  wifi-names-a-network-without-a-country =
    let
      sme = "net/wireless/sme.c";
      source = kernelSource self.packages.x86_64-linux.kernel [ sme ];
    in
    pkgs.runCommandLocal "wifi-names-a-network-without-a-country" { } ''
      ${lib.getExe pkgs.bash} ${./tests/ssid-before-country.sh} ${source}/${sme}
      touch $out
    '';
}
// onLinux (
  lib.mapAttrs kernelSettingsHold {
    kernel-settings-hold = self.packages.x86_64-linux.kernel;

    # Each piece of hardware builds its own kernel, with its own patches.
    kernel-settings-hold-dell-xps-14-da14260 =
      (bareNixos [ self.nixosModules.dell-xps-14-da14260 ]).config.boot.kernelPackages.kernel;
    kernel-settings-hold-microsoft-surface-laptop-4-1958 =
      (bareNixos [ self.nixosModules.microsoft-surface-laptop-4-1958 ]).config.boot.kernelPackages.kernel;
    # The Air's is cross-compiled from here, so no aarch64 builder is needed:
    # its configuration is the same either way, and only that is built.
    kernel-settings-hold-apple-macbook-air-m2-a2681 =
      (nixpkgs.lib.nixosSystem {
        modules = [
          self.nixosModules.apple-macbook-air-m2-a2681
          { nixpkgs.buildPlatform = "x86_64-linux"; }
        ];
      }).config.boot.kernelPackages.kernel;
    # And the M4 Pro's, cross-compiled the same way, which holds every
    # setting of Aurora's fragment it takes in.
    kernel-settings-hold-apple-macbook-pro-m4-pro-a3401 =
      (nixpkgs.lib.nixosSystem {
        modules = [
          self.nixosModules.apple-macbook-pro-m4-pro-a3401
          { nixpkgs.buildPlatform = "x86_64-linux"; }
        ];
      }).config.boot.kernelPackages.kernel;

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
