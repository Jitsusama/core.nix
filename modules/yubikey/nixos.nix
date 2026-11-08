# The YubiKey on Linux: its tools, from system.nix, and a touch that types
# nothing. The key's own OTP setting is one each key has to be told, by
# hand and once per key; refusing its keyboard here holds for every key
# plugged in, a new one too, from the disk prompt on.
{ pkgs, ... }:
let
  rules = builtins.readFile ./types-nothing.rules;
in
{
  imports = [ ./system.nix ];

  services.udev.packages = [
    (pkgs.writeTextDir "lib/udev/rules.d/70-yubikey-types-nothing.rules" rules)
  ];

  # The disk prompt is the initrd's, where a typed one-time password costs
  # a TPM try; these reach both the scripted initrd and systemd's.
  boot.initrd.services.udev.rules = rules;
}
