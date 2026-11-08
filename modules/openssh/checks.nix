# The checks of OpenSSH's server modules, which checks.nix gathers with every
# other folder's; its opening comment says what each kind of check proves.
{
  holds,
  lib,
  macWorkstation,
  workstation,
  ...
}:
{
  # A Mac's sshd asks a quiet caller every 15 s and lets it go after four
  # unanswered, in the file nix-darwin writes beside macOS's own.
  a-mac-sshd-lets-a-gone-caller-go =
    let
      written = macWorkstation.config.environment.etc."ssh/sshd_config.d/100-nix-darwin.conf";
    in
    holds "a-mac-sshd-lets-a-gone-caller-go"
      "A Mac's sshd never asks a quiet caller whether it's still there, so what a caller that went to sleep started there runs for hours."
      (
        written.enable
        && lib.hasInfix "ClientAliveInterval 15" written.text
        && lib.hasInfix "ClientAliveCountMax 4" written.text
      );

  # And takes only keys, never as root, in that same file.
  a-mac-sshd-takes-only-keys =
    let
      written = macWorkstation.config.environment.etc."ssh/sshd_config.d/100-nix-darwin.conf";
    in
    holds "a-mac-sshd-takes-only-keys"
      "A Mac's sshd takes a password or signs root in, so anyone on the laptop's network can guess their way in."
      (
        written.enable
        && lib.hasInfix "PasswordAuthentication no" written.text
        && lib.hasInfix "KbdInteractiveAuthentication no" written.text
        && lib.hasInfix "PermitRootLogin no" written.text
      );

  # So does every NixOS machine's.
  sshd-lets-a-gone-caller-go =
    let
      settings = workstation.config.services.openssh.settings;
    in
    holds "sshd-lets-a-gone-caller-go"
      "sshd never asks a quiet caller whether it's still there, so what a caller that went to sleep started there runs for hours."
      ((settings.ClientAliveInterval or null) == 15 && (settings.ClientAliveCountMax or null) == 4);

  # And takes only keys, never as root.
  sshd-takes-only-keys =
    let
      settings = workstation.config.services.openssh.settings;
    in
    holds "sshd-takes-only-keys"
      "sshd takes a password or signs root in, so anyone on the laptop's network can guess their way in."
      (
        settings.PasswordAuthentication == false
        && settings.KbdInteractiveAuthentication == false
        && settings.PermitRootLogin == "no"
      );
}
