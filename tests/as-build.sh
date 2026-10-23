# Starts a command as the Nix daemon starts a build, and prints its process
# id: as the first user id the daemon allocates, batch at nice 5, inside the
# daemon's cgroup.

# The first of the ids Nix allocates to builds, as its user and its group.
readonly build_user=872415232
readonly daemon=/sys/fs/cgroup/build.slice/nix-daemon.service

main() {
  setsid setpriv --reuid="$build_user" --regid="$build_user" --clear-groups \
    chrt -b 0 nice -n 5 "$@" >/dev/null 2>&1 </dev/null &
  echo "$!" >"$daemon/cgroup.procs"
  echo "$!"
}

main "$@"
