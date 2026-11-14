#!/usr/bin/env bash
# Tells chrony the time servers the networks name, as networkd keeps them in
# each link's state file, as one file of sources in chrony's source folder,
# and has chrony read it again only when it changed. It's run as networkd
# writes a link's state, and once as the machine starts. Its help takes the
# shape every command of ours does (docs/commands.md), by hand.

case "${1:-}" in
-h | --help)
  cat << 'EOF'
chrony-network-servers  tell chrony the networks' servers

  Writes the time servers networkd's links name as one
  file of chrony's sources, and has chrony read it again
  when it changed. A link's change and the machine's
  start run it.

EXAMPLES
  chrony-network-servers

EXIT
  0   told, or nothing changed
EOF
  exit 0
  ;;
*) ;;
esac
links=/run/systemd/netif/links
sources=/run/chrony-network/network.sources

shopt -s nullglob
states=("$links"/*)
servers=()
if ((${#states[@]} > 0)); then
  # Each link's NTP= line lists its servers, apart by spaces.
  mapfile -t servers < <(sed -n 's/^NTP=//p' "${states[@]}" | tr ' ' '\n' | sed '/^$/d' | sort -u)
fi

written="$sources.new"
: >"$written"
for server in "${servers[@]}"; do
  printf 'server %s iburst\n' "$server" >>"$written"
done
if cmp -s "$written" "$sources"; then
  rm "$written"
  exit 0
fi
mv "$written" "$sources"
echo "<6>the networks name ${#servers[@]} time servers: ${servers[*]}"
# chrony reads the folder as it starts, so one not running yet hears them then.
if ! chronyc reload sources >/dev/null; then
  echo "<5>chrony isn't answering yet, and reads them as it starts"
fi
