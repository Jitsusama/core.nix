#!/usr/bin/env bash
# What the checks that run the real tuple share: a folder of their own,
# made the current one, holding Tuple's home and every folder XDG names, so
# a script run anywhere leaves nothing behind and its stop reaches only a
# daemon started there, never a person's. Sourced first, before any tuple
# command runs, after the script has made its arguments whole paths, since
# the folder it moves into isn't the one they were given from.

here=$(mktemp -d)
# The folders are its own before the trap is set, so the stop can only
# reach a daemon started here. Those XDG names beyond the three Tuple reads
# are where they'd be under this home anyway, so an editor started with
# only the three, as tuple-completion.zsh starts them, agrees with the stop.
export HOME=$here XDG_DATA_HOME=$here/data XDG_RUNTIME_DIR=$here/run \
  XDG_CONFIG_HOME=$here/.config XDG_CACHE_HOME=$here/.cache \
  XDG_STATE_HOME=$here/.local/state
# Made before the trap too, so the stop never meets a runtime folder that
# isn't there and falls back to another.
if ! mkdir -m 700 "$here/run"; then
  rm -rf "$here"
  exit 1
fi

# A trap's own failure would end a passing run as failed, so the stop and
# the removal each say what went wrong instead. A stop cut off keeps the
# folder, since its daemon finds it there, and says how to stop it by hand;
# in a build, the build's end takes the daemon with it.
leave() {
  local stopped=0
  timeout -k 5 10 tuple stop > /dev/null 2>&1 || stopped=$?
  if [[ $stopped -eq 124 || $stopped -eq 137 ]]; then
    echo "tuple stop was cut off (status $stopped), so $here is kept for its daemon;" \
      "stop it with HOME=$here XDG_RUNTIME_DIR=$here/run XDG_DATA_HOME=$here/data" \
      "tuple stop, then remove $here" >&2
    return
  fi
  rm -rf "$here" || echo "couldn't remove $here" >&2
}
trap leave EXIT
cd "$here" || exit
