# shellcheck shell=bash
# sudo as Joel types it at a NixOS terminal, through run0 for its plain
# forms, so polkit asks: the shell's prompt, which names the command and
# takes his key's touch at his unlocked desk, and his password elsewhere
# or with no key in (modules/yubikey/once-in.sh). sudo itself reads only
# the password, and pam_u2f, which would let it take a touch, shows
# nothing on screen, so a touch an agent's sudo asked for would look like
# his own. A command, and a root shell alone, go to run0; anything sudo
# alone can say, such as another user, kept surroundings or a shell given
# a command, goes to sudo, which asks for the password as it always did.
# Its own help takes the shape every command of ours does
# (docs/commands.md), by hand, and says how to reach sudo's.
set -euo pipefail

case $# in
0) exec @sudo@ ;;
1)
  case $1 in
  -h | --help)
    cat << 'EOF'
sudo-by-touch  sudo through run0, so a key's touch answers

EXAMPLES
  sudo nixos-rebuild switch  run0 asks: a touch at the
                             unlocked desk, else a password
  sudo -i                    a root shell, through run0
  sudo -u someone true       sudo's own, with the password
  command sudo --help        sudo's own help

EXIT
  what run0 or sudo exits
EOF
    exit 0
    ;;
  -i | -s | --login | --shell) exec @run0@ ;;
  *) ;;
  esac
  ;;
*) ;;
esac
case $1 in
--)
  shift
  [ $# -gt 0 ] || exec @sudo@ --
  ;;
-*) exec @sudo@ "$@" ;;
*) ;;
esac
exec @run0@ -- "$@"
