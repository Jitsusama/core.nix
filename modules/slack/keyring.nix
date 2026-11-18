# Prints pi's Slack sign-in from the desktop's keyring, where pi has kept it
# since moving sign-ins there, for every reader of it: slack-unreads,
# slack-people and pineapple's Slack reader. While the keyring holds none it
# fails, so the old file is read instead (lib's `Account`); while the keyring
# is locked, its unlock prompt dismissed or with no prompter to show it, it
# ends 75 (sign-in-program's `LOCKED`), so a reader keeps the sign-in it
# last read and rests rather than taking Joel for signed out. secret-tool
# ends 1 saying nothing for both, so the keyring is asked which it was.
# secret-tool's output goes to the program alone.
{
  lib,
  writeShellScript,
  libsecret,
  systemd,
}:
writeShellScript "slack-sign-in" ''
  ${lib.getExe' libsecret "secret-tool"} lookup \
    service agentic-harness username slack-credentials && exit 0
  locked=$(${lib.getExe' systemd "busctl"} --user get-property org.freedesktop.secrets \
    /org/freedesktop/secrets/aliases/default org.freedesktop.Secret.Collection Locked \
    2>/dev/null)
  [ "$locked" = "b true" ] && exit 75
  exit 1
''
