# Prints pi's Slack sign-in from the desktop's keyring, where pi has kept it
# since moving sign-ins there, for every reader of it: slack-unreads,
# slack-people and pineapple's Slack reader. While the keyring holds none it
# fails, so the old file is read instead (src/account.rs's `Account`);
# while it holds the sign-in locked, its unlock prompt dismissed or with no
# prompter to show it, it ends 75 (sign-in-program's `LOCKED`), so a reader
# keeps the sign-in it last read and rests rather than taking Joel for
# signed out.
# The program is sign-in-program/keyring.sh, where its cases run it; this
# fills in the two programs it asks.
{
  lib,
  writeShellScript,
  libsecret,
  systemd,
}:
writeShellScript "slack-sign-in" (
  lib.replaceStrings
    [ "@secretTool@" "@busctl@" ]
    [ (lib.getExe' libsecret "secret-tool") (lib.getExe' systemd "busctl") ]
    (builtins.readFile ./sign-in-program/keyring.sh)
)
