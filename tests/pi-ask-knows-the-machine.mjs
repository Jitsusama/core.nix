// pi-ask's digest (modules/pi/ask-digest.ts, the first argument) over a
// made-up machine: its battery and power, the disk guard's word on its disk,
// its network links, what waits on the person, and its quests. It names the
// person and the time, keeps to the quests driven or worked this past week,
// newest first, with the whole of each one's newest Journey entry, and
// leaves out what the machine doesn't have. It goes after the system prompt
// as each question starts.
import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

const { digest, default: register } = await import(process.argv[2]);

const QUEST_LINE = /^- [A-Z0-9]{6} /;

const root = mkdtempSync(join(tmpdir(), "ask-digest-"));
const write = (path, content) => {
  mkdirSync(dirname(join(root, path)), { recursive: true });
  writeFileSync(join(root, path), content);
};
const places = {
  home: join(root, "home"),
  state: join(root, "state"),
  runtime: join(root, "run"),
  quests: join(root, "quests"),
  sys: join(root, "sys"),
};
mkdirSync(places.home, { recursive: true });
const now = new Date("2026-10-26T13:45:00Z");

write("sys/power_supply/BAT0/type", "Battery\n");
write("sys/power_supply/BAT0/capacity", "64\n");
write("sys/power_supply/BAT0/status", "Discharging\n");
write("sys/power_supply/AC/type", "Mains\n");
write("sys/power_supply/AC/online", "0\n");
write("sys/net/wlan0/operstate", "up\n");
write("sys/net/wlan0/device/vendor", "0x8086\n");
write("sys/net/wlan0/wireless/.keep", "");
write("sys/net/lo/operstate", "unknown\n");
write("sys/net/tunnel0/operstate", "up\n");
write(
  "run/disk-guard/status.json",
  JSON.stringify({ says: "The disk is healthy, with 300 GiB free." }),
);
write(
  "state/mission/queue.json",
  JSON.stringify({ items: [{ title: "Type ~/s at the other laptop", when: "now" }] }),
);
write(
  "state/mission/waiting/ABCDEF.json",
  JSON.stringify({
    quest: "QEST-20261025-ABCDEF",
    items: [{ title: "Read the boot screen", when: "at the restart" }],
  }),
);
write("state/mission/waiting/broken.json", "{ half");

const quest = (id, front, journey, touched) => {
  write(`quests/${id}/README.md`, `---\nid: ${id}\n${front}\n---\n\n# The Title of ${id}\n`);
  if (journey) {
    write(`quests/${id}/JOURNEY.md`, `# 🌄 Journey\n\n${journey}\n- **2026-10-01**: Created.\n`);
  }
  const at = touched / 1000;
  utimesSync(join(root, "quests", id, "README.md"), at, at);
};
quest(
  "QEST-20261025-AAAAAA",
  "short: Harness\nstatus: active\npriority: active\nupdated: 2026-10-26",
  "- **2026-10-26**: 09:30: merged the fix,\n  which spans two lines.",
  now.getTime() - 60_000,
);
quest(
  "QEST-20261025-BBBBBB",
  "status: blocked\npriority: driving\nupdated: 2026-10-25",
  "- **2026-10-25**: Waiting on a login.",
  now.getTime() - 3_600_000,
);
quest(
  "QEST-20261025-CCCCCC",
  "status: concluded\npriority: active\nupdated: 2026-10-26",
  "",
  now.getTime(),
);
quest(
  "QEST-20261025-DDDDDD",
  "status: active\npriority: bench\nupdated: 2026-10-26",
  "",
  now.getTime(),
);
quest(
  "QEST-20260901-EEEEEE",
  "status: active\npriority: driving\nupdated: 2026-09-01",
  "",
  now.getTime(),
);

const said = digest(now, places, "Joel Gerber");
const lines = said.split("\n");
assert.match(lines[0], /^What's true right now/);
assert.ok(lines.includes("Asking: Joel Gerber, whose machine this is."));
assert.match(said, /^Now: Monday, October 26, 2026 at \d\d:45/m);
assert.match(
  said,
  /^Machine: .+; battery 64%, discharging, not on power; disk: The disk is healthy, with 300 GiB free\.; network: wlan0 \(Wi-Fi\)\.$/m,
);
assert.ok(lines.includes("Waiting on them:"));
assert.ok(lines.includes("- Type ~/s at the other laptop, now"));
assert.ok(lines.includes("- Read the boot screen (ABCDEF), at the restart"));
const listed = lines.filter((line) => QUEST_LINE.test(line));
assert.deepEqual(listed, [
  "- AAAAAA Harness, The Title of QEST-20261025-AAAAAA (active): 09:30: merged the fix, which spans two lines.",
  "- BBBBBB The Title of QEST-20261025-BBBBBB (blocked, driving): Waiting on a login.",
]);

const bare = {
  ...places,
  runtime: undefined,
  state: join(root, "none"),
  quests: join(root, "none"),
  sys: join(root, "none"),
};
const plain = digest(now, bare);
assert.doesNotMatch(plain, /Asking:|battery|network|Quests/);
assert.match(plain, /disk: [\d.]+ GiB free/);
assert.ok(plain.split("\n").includes("Nothing waits on them."));

let handler;
register({
  on: (event, run) => {
    if (event === "before_agent_start") {
      handler = run;
    }
  },
});
const { systemPrompt } = handler({ systemPrompt: "You answer quick questions." });
assert.match(systemPrompt, /^You answer quick questions\.\n\nWhat's true right now/);
