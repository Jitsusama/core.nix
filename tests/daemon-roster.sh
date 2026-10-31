# shellcheck shell=bash
# Stands in for a source that says everyone it knows unasked, as Tuple's
# does, run by the people daemon on People's line protocol (modules/people):
# Eli, a favourite, and Hal, whom only a search finds, so tests/daemon.qml
# can tell a watch of those listed and a search's answer from what's held;
# and Ivy, whom it no longer knows once it's asked anything, so the
# client lets go of one no source knows. It reads what the daemon asks to
# its end.
say() {
  printf '{"email":"%s","id":%s,"name":"%s","status":"unknown","favourite":%s,"recent":false}\n' "$@"
}
echo '{"roster":true}'
say eli@example.com 1 'Eli Fenwick' true
say hal@example.com 2 'Hal Ormond' false
say ivy@example.com 3 'Ivy Penrose' false
while read -r _; do
  echo '{"email":"ivy@example.com","id":3,"gone":true}'
done
