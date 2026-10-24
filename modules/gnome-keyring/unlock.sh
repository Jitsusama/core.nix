# shellcheck shell=bash
# Unlocks the login keyring as the session starts, with a random password
# sealed to the TPM for this account on this install, making both the first
# time, then runs the keyring's daemon in this service's place.

sealed="$STATE_DIRECTORY/login.cred"
keyring="${XDG_DATA_HOME:-$HOME/.local/share}/keyrings/login.keyring"

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
    | systemd-creds encrypt --user --name=login - "$sealed.new"
  mv "$sealed.new" "$sealed"
fi

# Decrypted before the daemon starts, so a decryption that fails stops
# here, and no child is left for the daemon to inherit and never reap.
# Taking the output drops the newline base64 ended the password with,
# and the here-string puts exactly one back.
password=$(systemd-creds decrypt --user --name=login "$sealed" -)

# The daemon reads the password from its standard input, unlocks the
# login keyring with it, and makes the keyring when there's none.
exec /run/wrappers/bin/gnome-keyring-daemon \
  --replace --foreground --unlock --components=secrets \
  <<< "$password"
