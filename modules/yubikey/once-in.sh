# shellcheck shell=bash
# Says whether polkit may take a YubiKey's touch for the account signing
# in, which is only once the person is already in: their seat's session is
# in front, local and unlocked, and the prompt came from it rather than from
# an ssh login or a text console. pam_exec runs it just before pam_u2f in
# polkit's sign-in, as root inside the helper systemd starts for each
# prompt; succeeding lets the touch be asked, and anything else skips it,
# so the password is asked as if no key were named. Anything it can't read
# counts as no.
#
# It reads the lock's word in the account's runtime folder, which the shell
# writes (modules/quickshell/Lock.qml): "unlocked" once it's let go, and
# "locked" from the moment every screen shows it.
set -euo pipefail

# pam_exec sets PAM_USER to the account signing in.
# shellcheck disable=SC2154
uid=$(id -u -- "$PAM_USER")

# Who asked: systemd names each helper by the process at the other end of
# its socket, <connection>-<cookie>-<pid>_<pidfd>-<uid>, so an account can't
# say it's another or come from somewhere else.
unit=$(grep -o 'polkit-agent-helper@[0-9_-]*\.service' /proc/self/cgroup)
instance=${unit#*@}
instance=${instance%.service}
asker_uid=${instance##*-}
rest=${instance%-*}
pid=${rest##*-}
pid=${pid%%_*}
[ "$asker_uid" = "$uid" ]

# The seat's session, as logind keeps it for the account: a desktop's, in
# front, at the machine itself, and not said to be locked. A text console
# is never it, even the only session there is, since nothing locks one.
seat=$(loginctl show-user "$uid" --property=Display --value)
[ -n "$seat" ]
said=$(loginctl show-session "$seat" --property=Type --property=Active --property=Remote \
  --property=LockedHint)
grep -qx -e 'Type=wayland' -e 'Type=x11' <<<"$said"
grep -qx 'Active=yes' <<<"$said"
grep -qx 'Remote=no' <<<"$said"
grep -qx 'LockedHint=no' <<<"$said"

# The asker belongs to that session or to the account's own services, which
# is where the desktop's programs run; a login of its own, as over ssh or at
# a text console, is another session, and gets the password.
cgroup=$(cat "/proc/$pid/cgroup")
if [[ $cgroup =~ /session-([^/]+)\.scope ]]; then
  [ "${BASH_REMATCH[1]}" = "$seat" ]
else
  [[ $cgroup == *"/user@$uid.service/"* ]]
fi

# And the lock is let go.
said=$(cat "/run/user/$uid/quickshell-locked")
[ "$said" = unlocked ]
