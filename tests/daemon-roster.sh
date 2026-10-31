# shellcheck shell=bash
# Stands in for a source that says everyone it knows unasked, as Tuple's
# does, run by the people daemon on People's line protocol (modules/people):
# Eli, a favourite, and Hal, whom only a search finds, so tests/daemon.qml
# can tell a watch of those listed and a search's answer from what's held.
# It reads what the daemon asks to its end, answering nothing.
say() {
  printf '{"email":"%s","id":%s,"name":"%s","status":"unknown","favourite":%s,"recent":false}\n' "$@"
}
echo '{"roster":true}'
say eli@example.com 1 'Eli Fenwick' true
say hal@example.com 2 'Hal Ormond' false
while read -r _; do :; done
