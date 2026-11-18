# The harness's files a realm's node serves whoever reads it (home.nix), the
# launcher first, each as a kind of records: the realm's open quests and its
# ended ones, each list a file the harness keeps, the sessions to restore,
# the runs of subagents and reviews its sessions started, a file each, what
# its sessions landed in the last 30 days, and the talks in its sessions'
# steer inboxes, a record each of its lines; and on the home realm's node the
# machine's waiting, verdicts and queue, every answer the void has heard, a
# line each, and the balance service's latest reading, which no realm owns,
# so nothing is heard twice. The queue is in the login's runtime folder,
# which the node finds, as a Mac's differs by account: each job, and the
# queue whole as how the machine stands, named for it. A file not written yet
# is served as holding nothing. Its own file, as what a node serves changes
# for its own reasons, apart from how the node runs.
{
  config,
  cfg,
  lib,
}:
realm:
let
  state = config.xdg.stateHome;
  home = realm == cfg.realm;
  harness =
    if home then "${state}/pi/agentic-harness.pi" else "${state}/pi/agentic-harness.pi/realms/${realm}";
in
{
  quest = {
    file = "${harness}/quest-workflow/quests.json";
    records = "quests";
    key = "id";
  };
  ended-quest = {
    file = "${harness}/quest-workflow/ended.json";
    records = "quests";
    key = "id";
  };
  restore = {
    file = if home then "${state}/pi/restore.json" else "${harness}/restore.json";
    records = "sessions";
    key = "id";
  };
  run = {
    folder = "${harness}/mesh-workflow/runs";
    records = "runs";
    key = "id";
  };
  landing = {
    file = "${harness}/mesh-workflow/landings.json";
    records = "landings";
    key = "key";
  };
  talk.talks = "${harness}/steer";
}
// lib.optionalAttrs home {
  waiting = {
    folder = "${state}/mission/waiting";
    records = "items";
    # A wait without an id, or with an empty one, is keyed by its title,
    # so it no longer hides its quest's whole file.
    key = "id";
    or = "title";
    by = "quest";
  };
  verdict = {
    file = "${state}/mission/verdicts.json";
    records = "items";
  };
  job = {
    file = "jobs/queue.json";
    within = "runtime";
    records = "jobs";
    key = "id";
  };
  queue = {
    file = "jobs/queue.json";
    within = "runtime";
    whole = true;
  };
  void-answers = {
    file = "${state}/void/answers.jsonl";
    whole = true;
  };
  # The balance service's latest reading of the realm it reads, kept whole
  # each minute, so the map draws each machine's share of it.
  balance = {
    file = "${state}/mycelium/balance/now.json";
    whole = true;
  };
}
