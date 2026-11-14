# Shapes what the realm's node says now into the one file the mycelium site's
# map reads, now.json, from the three answers mycelium-map.sh passes in:
# $n from `mycelium networks -o json`, $m from `mycelium machines` and $g
# from `mycelium messages`. The networks and their cells pass as mycelium
# draws them, so the map and `mycelium networks` show one record; the
# machines add what their cells leave out (memory, disk, power) and the
# messages only their senders and groups, never their words.
{
  at: (now | todate),
  networks: [$n[] | select(.kind == "network")],
  cells: [$n[] | select(.kind == "cell" and .type == "session")],
  machines: [
    $m[0].Ok.Machines[]
    | {
        name, state, model_name, os, system,
        cores: .cpu.cores, memory, disk, disk_free, disk_band, power,
        available: (.room.memory_available // null),
        pressure: (.room.pressure // null),
        over: (.room.over // []),
        measured_at
      }
  ],
  messages: [
    $g[0].Ok.Messages[]
    | {at, from, intent, groups}
  ]
}
