# The YubiKey is the way back in when the machine's own keys can't be used,
# never the everyday key: on Linux the disk's recovery slot, and on any machine
# a spare SSH key for logins and signing, an age identity that opens backups,
# and single sign-on. All of it is FIDO2, which systemd's own udev rules
# (60-fido-id and 70-uaccess) let the signed-in account reach on Linux; a Mac
# needs nothing set up for it. Nothing uses its OpenPGP or smart card
# applications.
{ pkgs, ... }:
{
  environment.systemPackages = [
    # ykman, to set the FIDO2 PIN and turn off the applications nothing uses.
    pkgs.yubikey-manager
    # fido2-token, to list the keys and the credentials on them.
    pkgs.libfido2
    # age encrypting to the YubiKey through FIDO2's hmac-secret.
    pkgs.age-plugin-fido2-hmac
  ];
}
