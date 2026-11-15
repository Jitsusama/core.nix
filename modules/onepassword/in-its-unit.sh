# shellcheck shell=bash disable=SC2154 # The app comes from home.nix's header.
# 1Password opened from its desktop entry (home.nix): its unit is started
# first, so the app runs in the unit, which says it's open, and this second
# run hands the unit's app its window, as 1Password runs once a session.
# The header written above this gives app, 1Password's own program.
#
# The unit's app can die making that window, as it did on 11 November
# binding a screen gone since it started, and the unit brings it back
# without one, so the hand-off dies with it and the opening shows nothing.
# So where the unit started again under the hand-off, the app it started
# is asked for its window once more, and only once.
#
# Its help takes the shape every command of ours does (docs/commands.md),
# by hand; every other word goes to 1Password.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
1password-in-its-unit  open 1Password's window in its unit

  Starts onepassword.service, then hands the app there
  its window, asking once more if the unit restarted
  under it. 1Password's desktop entry runs it, and its
  words go to 1Password.

EXAMPLES
  1password-in-its-unit  open 1Password's window

EXIT
  0     the window was handed over
  else  1Password's own code for the hand-off
EOF
  exit 0
  ;;
*) ;;
esac

invocation() {
  systemctl --user show --property=InvocationID --value onepassword.service || true
}

systemctl --user start onepassword.service || true
before=$(invocation)
handed=0
"$app" "$@" || handed=$?
# A start waits out a restart already under way, so the app asked again is
# the unit's new one, ready for it.
systemctl --user start onepassword.service || true
now=$(invocation)
if [ "$now" != "$before" ]; then
  exec "$app" "$@"
fi
exit "$handed"
