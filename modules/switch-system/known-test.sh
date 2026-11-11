# shellcheck shell=bash
# The test of switch-system-known.sh: a system the profile holds is known,
# unless its PAM lets a YubiKey's touch in alone, and one it doesn't hold
# never is. The systems are stand-ins, a pam.d folder each, in a profile of
# their own in the working folder.
#
# usage: bash known-test.sh switch-system-known.sh
set -euo pipefail

known=$1
export SWITCH_SYSTEM_PROFILES=$PWD/profiles
mkdir -p profiles

# A system with one PAM service, held as the profile's generation $3.
held() {
  local name=$1 service=$2 generation=$3 text=$4
  mkdir -p "$PWD/$name/etc/pam.d"
  printf '%s\n' "$text" >"$PWD/$name/etc/pam.d/$service"
  ln -s "$PWD/$name" "profiles/system-$generation-link"
}

# Fails unless switch-system-known says $2 of the system named $1.
says() {
  local unit status=0
  unit="switch-system@$(systemd-escape --path "$PWD/$1").service"
  bash "$known" "$unit" || status=$?
  if [[ $status != "$2" ]]; then
    echo "switch-system-known said $status for $1, not $2" >&2
    exit 1
  fi
}

held plain login 1 'auth required pam_unix.so'
held gated polkit-1 2 $'auth [success=ok default=1] pam_exec.so quiet yubikey-once-in\nauth sufficient pam_u2f.so'
held alone polkit-1 3 'auth sufficient pam_u2f.so'
held at-login login 4 'auth sufficient pam_u2f.so'
mkdir -p never/etc/pam.d

says plain 0
says gated 0
says alone 1
says at-login 1
says never 1
