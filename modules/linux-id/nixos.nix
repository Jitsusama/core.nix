# A passkey the TPM holds, for the web: linux-id (package.nix) plays a FIDO
# security key over uhid, so a browser takes it for one, and makes each
# site's key in the TPM, where it can't be copied off the laptop. Before a
# key signs, it asks whether Joel is there through pinentry-askpass
# (pinentry.rs), which asks the shell's prompt (SSH_ASKPASS). It confirms
# presence only, never who's there, so a site that insists on knowing is
# refused rather than told a lie, and the YubiKeys are that site's keys.
# Chrome's probe of which key was touched it refuses without asking
# (package.nix), so such a sign-in goes on to the phone or a YubiKey.
#
# It runs in every account. The signed-in account may make HID devices, as
# linux-id needs to, so a program it runs could pose as a keyboard too.
{ lib, pkgs, ... }:
let
  linuxId = pkgs.callPackage ./package.nix { };

  # The standard library alone, so it builds with rustc and no crates.
  pinentry = pkgs.runCommandCC "pinentry-askpass" { nativeBuildInputs = [ pkgs.rustc ]; } ''
    mkdir -p $out/bin
    rustc --edition 2024 -C opt-level=3 -D warnings \
      -o $out/bin/pinentry-askpass ${./pinentry.rs}
  '';

  # Before the seat's rules, which give the signed-in account what's tagged.
  # The TPM is already the account's, through the tss group (modules/tpm).
  uhid = pkgs.writeTextFile {
    name = "linux-id-uhid";
    destination = "/etc/udev/rules.d/70-linux-id.rules";
    text = ''
      KERNEL=="uhid", SUBSYSTEM=="misc", TAG+="uaccess"
    '';
  };
in
{
  imports = [ ../tpm/nixos.nix ];

  # Loaded at boot, so udev tags the device before anything opens it: opened
  # first, the kernel would load it with no rule having run.
  boot.kernelModules = [ "uhid" ];
  services.udev.packages = [ uhid ];

  systemd.user.services.linux-id = {
    description = "A security key the TPM holds";
    documentation = [ "https://github.com/matejsmycka/linux-id" ];
    wantedBy = [ "default.target" ];
    # The TPM's resource manager is all it needs; systemd's tpm2 condition
    # also asks the firmware, which a machine booted without UEFI can't say.
    unitConfig.ConditionPathExists = "/dev/tpmrm0";
    environment.PINENTRY_PATH = lib.getExe' pinentry "pinentry-askpass";
    serviceConfig = {
      ExecStart = lib.getExe linuxId;
      Restart = "on-failure";
      RestartSec = 5;
      # Passkeys a site keeps on the key are written here, which systemd
      # leaves writable.
      ConfigurationDirectory = "linux-id";
      # It needs the TPM, the device it makes, its own folder, and the
      # runtime directory, where the askpass asks the shell.
      NoNewPrivileges = true;
      ProtectSystem = "strict";
      ProtectHome = "read-only";
      ReadWritePaths = [ "%t" ];
      PrivateTmp = true;
      PrivateNetwork = true;
      RestrictAddressFamilies = [ "AF_UNIX" ];
      RestrictNamespaces = true;
      RestrictRealtime = true;
      RestrictSUIDSGID = true;
      LockPersonality = true;
      ProtectClock = true;
      ProtectHostname = true;
      ProtectKernelLogs = true;
      ProtectKernelModules = true;
      ProtectKernelTunables = true;
      ProtectControlGroups = true;
    };
  };
}
