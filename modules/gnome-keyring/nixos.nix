# GNOME Keyring keeps the secrets programs store through the Secret Service,
# such as Chrome's and Slack's logins. Joel signs in once, at boot, with the
# disk's PIN, so PAM never sees a password to unlock the keyring with, and
# gnome-keyring would ask for one in a prompt of its own, outside the theme.
#
# The keyring's password is a random one instead, sealed to the TPM for this
# account on this install. A user service unlocks the keyring with it as the
# session starts, so nothing ever asks, and the keyring on disk stays
# encrypted: a copy of it opens nowhere else.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  creds = lib.getExe' config.systemd.package "systemd-creds";

  unlock = pkgs.writeShellApplication {
    name = "gnome-keyring-unlock";
    runtimeInputs = [ pkgs.coreutils ];
    text = ''
      sealed="$STATE_DIRECTORY/login.cred"
      keyring="''${XDG_DATA_HOME:-$HOME/.local/share}/keyrings/login.keyring"

      if [ ! -e "$sealed" ]; then
        # A keyring made before this service has a password nobody sealed,
        # so a new one wouldn't open it.
        if [ -e "$keyring" ]; then
          echo "gnome-keyring-unlock: $keyring has a password that isn't sealed;" \
            "move it aside to start a new keyring." >&2
          exit 1
        fi
        # systemd seals it with the TPM and the install's own credential key,
        # for this account alone, so it opens only here.
        head -c 32 /dev/urandom | base64 \
          | ${creds} encrypt --user --name=login - "$sealed.new"
        mv "$sealed.new" "$sealed"
      fi

      # Decrypted before the daemon starts, so a decryption that fails stops
      # here, and no child is left for the daemon to inherit and never reap.
      # Taking the output drops the newline base64 ended the password with,
      # and the here-string puts exactly one back.
      password=$(${creds} decrypt --user --name=login "$sealed" -)

      # The daemon reads the password from its standard input, unlocks the
      # login keyring with it, and makes the keyring when there's none.
      exec /run/wrappers/bin/gnome-keyring-daemon \
        --replace --foreground --unlock --components=secrets \
        <<<"$password"
    '';
  };

  # Asking for the Secret Service starts the unit below, not a daemon of its
  # own, so a program that asks before the keyring is unlocked waits for it
  # instead of finding it locked. The session bus reads system-path's files
  # before gnome-keyring's, and hiPrio puts these there in their place.
  activation = pkgs.runCommand "gnome-keyring-activation" { } ''
    mkdir -p $out/share/dbus-1/services
    for name in org.freedesktop.secrets org.gnome.keyring; do
      printf '[D-BUS Service]\nName=%s\nExec=%s\nSystemdService=%s\n' \
        $name ${pkgs.coreutils}/bin/false gnome-keyring.service \
        > $out/share/dbus-1/services/$name.service
    done
  '';
in
{
  imports = [ ../tpm/nixos.nix ];

  services.gnome.gnome-keyring.enable = true;
  environment.systemPackages = [ (lib.hiPrio activation) ];

  # It starts with the session but doesn't hold it back: a slow TPM can take
  # over a second to unseal the password, and niri needs no secrets.
  systemd.user.services.gnome-keyring = {
    description = "GNOME Keyring, unlocked with its password from the TPM";
    wantedBy = [ "graphical-session-pre.target" ];
    serviceConfig = {
      # Ready once it holds the Secret Service's name, which it takes only
      # after unlocking, so nothing in the session reaches a locked keyring.
      Type = "dbus";
      BusName = "org.freedesktop.secrets";
      ExecStart = lib.getExe unlock;
      StateDirectory = "gnome-keyring";
      Restart = "on-failure";
    };
  };

  # gnome-keyring's own autostart entry would start a second daemon at login,
  # which finds this one holding the Secret Service and exits. Its pkcs11 entry
  # stays, since it adds that component to this daemon.
  systemd.user.units."app-gnome\\x2dkeyring\\x2dsecrets@autostart.service".enable = false;
}
