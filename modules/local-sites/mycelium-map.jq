# Shapes what the realm's node says now into the one file the mycelium site's
# map reads, now.json, from the four answers mycelium-map.sh passes in:
# $s from `mycelium sessions`, $w from `mycelium swarm --json`, $m from
# `mycelium machines` and $g from `mycelium messages`. It keeps only what the
# map draws: the live sessions, ended ones left out, each team, each machine,
# and the latest messages' senders and groups, never their words. Nothing
# here judges; the page does, so the model has one reader.
{
  at: (now | todate),
  sessions: [
    $s[0].Ok.Sessions[]
    | select(.state == "busy" or .state == "idle")
    | {
        id, address, title, machine, state, quest,
        since: (.since // .presence.since),
        step: (.step // .presence.step),
        wants: (.wants.said // null),
        paused: (.presence.paused == true),
        parked: (.presence.state == "parked"),
        project: (.presence.questInfo.title // null),
        repo: (.presence.repo // null),
        groups: [.groups[] | select(startswith("peer.") | not)],
        lead_of: (.presence.orchestrates // null),
        cost_today: (.presence.usage["pi.cost.usd"].today // null),
        context: (.presence.usage.context // null),
        turns_today: (.presence.usage.turns.today // null),
        last_turn: (.presence.usage.turns.last.at // null)
      }
  ],
  teams: [
    $w[]
    | select(.kind == "team")
    | {key: (.key | ltrimstr("team:")), title, within, lead, members, counts, worst, bridges, landed, liaisons}
  ],
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
