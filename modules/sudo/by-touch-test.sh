# shellcheck shell=bash
# Proves by-touch.sh hands sudo's plain forms to run0 and leaves every
# other to sudo itself. Usage: by-touch-test <wrapper>, where the wrapper
# was built in front of stand-ins for both that echo their name and what
# they're given.
set -euo pipefail
by_touch=$1

# says <what the stand-ins should say> <the call...>
says() {
  local want=$1
  shift
  local got
  got=$("${by_touch}" "$@")
  if [[ "${got}" != "${want}" ]]; then
    echo "sudo $* became '${got}', not '${want}'"
    exit 1
  fi
}

# A command, as Joel types it most, goes to run0, its words whole.
says "run0 -- nixos-rebuild switch --flake ." nixos-rebuild switch --flake .
says "run0 -- cat /etc/shadow" cat /etc/shadow
says "run0 -- ls -la /root" -- ls -la /root
says "run0 -- touch a file" touch "a file"
# A root shell is run0's own.
says "run0" -i
says "run0" -s
says "run0" --login
# What sudo alone can say, or a shell given a command, stays sudo's.
says "sudo -u someone true" -u someone true
says "sudo -E env" -E env
says "sudo -i whoami" -i whoami
says "sudo -k" -k
says "sudo --" --
says "sudo"
# Its own help is its, naming it first and how to reach sudo's.
help=$("${by_touch}" --help)
if [[ "${help}" != sudo-by-touch* || "${help}" != *"command sudo --help"* ]]; then
  echo "sudo --help didn't answer sudo-by-touch's own help"
  exit 1
fi
says "sudo -h -u someone" -h -u someone
echo "every case held"
