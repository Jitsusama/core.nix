/**
 * What pi-ask knows as each question starts: a short digest read from this
 * machine's own files at that moment, put after the system prompt, so an
 * answer about the person, the time, the machine or the agents' work can
 * come from what's true now. Only local files are read, never a network or
 * a work machine's tooling, so it reads the same on any machine, and what a
 * machine doesn't have is left out rather than guessed.
 *
 * - who asks ($ASK_PERSON), and the date and time;
 * - the machine: its name, battery, disk (what the disk guard says, else
 *   the free space), and the network links that are up;
 * - what waits on the person: the orchestrator's queue and each quest's
 *   own waiting file ($XDG_STATE_HOME/mission);
 * - the quests being driven or worked and updated this past week, newest
 *   first: short name, status, and the newest entry of each one's Journey.
 */
import { closeSync, openSync, readdirSync, readFileSync, readSync, statfsSync, statSync } from "node:fs";
import { homedir, hostname } from "node:os";
import { join } from "node:path";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

/** The most quests listed, the most recently updated kept. */
const MOST_QUESTS = 30;
/** How long since a quest was last updated it's still listed. */
const QUIET_DAYS = 7;
const DAY_MS = 24 * 60 * 60 * 1_000;
/** How much of a quest's newest Journey line is kept. */
const JOURNEY_CHARS = 140;
/** How much of a README is read for its front matter and title. */
const README_BYTES = 16_384;
/** How much of a Journey is read for its newest line. */
const JOURNEY_BYTES = 4_096;
const GIB = 1024 ** 3;

/** Where the digest's sources are, so a test can point them elsewhere. */
export interface Places {
  home: string;
  state: string;
  runtime?: string;
  quests: string;
  sys: string;
}

/** The places as this account has them. */
export function placesHere(env: NodeJS.ProcessEnv = process.env): Places {
  const home = env.HOME || homedir();
  const share = env.XDG_DATA_HOME || join(home, ".local", "share");
  return {
    home,
    state: env.XDG_STATE_HOME || join(home, ".local", "state"),
    runtime: env.XDG_RUNTIME_DIR,
    quests: join(share, "pi", "agentic-harness.pi", "quest-workflow", "quests"),
    sys: "/sys/class",
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** A file's text, or undefined when there's none to read. */
function text(path: string, bytes?: number): string | undefined {
  try {
    if (bytes === undefined) return readFileSync(path, "utf8");
    const fd = openSync(path, "r");
    try {
      const buffer = Buffer.alloc(bytes);
      const read = readSync(fd, buffer, 0, bytes, 0);
      return buffer.subarray(0, read).toString("utf8");
    } finally {
      closeSync(fd);
    }
  } catch {
    // Missing or unreadable: the digest leaves that part out.
    return undefined;
  }
}

function json(path: string): unknown {
  const read = text(path);
  if (read === undefined) return undefined;
  try {
    return JSON.parse(read);
  } catch {
    // Half-written or not JSON: left out, as if absent.
    return undefined;
  }
}

function names(folder: string): string[] {
  try {
    return readdirSync(folder);
  } catch {
    // No such folder on this machine.
    return [];
  }
}

function clip(line: string, most: number): string {
  const flat = line.replace(/\s+/g, " ").trim();
  return flat.length <= most ? flat : `${flat.slice(0, most - 1)}…`;
}

/** The battery and whether it's on power, from Linux's power supplies. */
function power(places: Places): string | undefined {
  const supplies = join(places.sys, "power_supply");
  const said: string[] = [];
  // Whether a charger is plugged in; Linux calls its supply type "Mains".
  let charger: boolean | undefined;
  for (const supply of names(supplies)) {
    const kind = text(join(supplies, supply, "type"))?.trim();
    if (kind === "Battery") {
      const capacity = text(join(supplies, supply, "capacity"))?.trim();
      const status = text(join(supplies, supply, "status"))?.trim().toLowerCase();
      if (capacity) said.push(`battery ${capacity}%${status ? `, ${status}` : ""}`);
    } else if (kind === "Mains") {
      charger = charger || text(join(supplies, supply, "online"))?.trim() === "1";
    }
  }
  if (charger !== undefined) said.push(charger ? "on power" : "not on power");
  return said.length ? said.join(", ") : undefined;
}

/** The disk, as the disk guard says it, else the home's free space. */
function disk(places: Places): string | undefined {
  const guard = places.runtime ? json(join(places.runtime, "disk-guard", "status.json")) : undefined;
  if (isRecord(guard) && typeof guard.says === "string") return guard.says;
  try {
    const stats = statfsSync(places.home);
    return `${((stats.bavail * stats.bsize) / GIB).toFixed(1)} GiB free`;
  } catch {
    // No way to ask on this machine.
    return undefined;
  }
}

/** The physical network links that are up, Wi-Fi or wired. */
function network(places: Places): string | undefined {
  const links = join(places.sys, "net");
  const up = names(links).filter(
    (link) =>
      text(join(links, link, "operstate"))?.trim() === "up" &&
      names(join(links, link, "device")).length > 0,
  );
  if (up.length === 0) return names(links).length ? "no physical link up" : undefined;
  return up
    .map((link) => `${link} (${names(join(links, link, "wireless")).length || link.startsWith("wl") ? "Wi-Fi" : "wired"})`)
    .join(", ");
}

/** Each thing waiting on the person: the orchestrator's, then each quest's. */
function waiting(places: Places): string[] {
  const mission = join(places.state, "mission");
  const files = [join(mission, "queue.json"), ...names(join(mission, "waiting")).map((name) => join(mission, "waiting", name))];
  const lines: string[] = [];
  for (const file of files) {
    const read = json(file);
    if (!isRecord(read) || !Array.isArray(read.items)) continue;
    const from = typeof read.quest === "string" ? ` (${read.quest.slice(-6)})` : "";
    for (const item of read.items) {
      if (!isRecord(item) || typeof item.title !== "string") continue;
      const when = typeof item.when === "string" ? `, ${item.when}` : "";
      lines.push(`- ${clip(item.title, 160)}${from}${when}`);
    }
  }
  return lines;
}

interface Quest {
  line: string;
  updated: number;
}

function field(front: string, name: string): string | undefined {
  return new RegExp(`^${name}: *(.+)$`, "m").exec(front)?.[1]?.trim();
}

/** The quests being driven or worked lately, newest first, one line each. */
function quests(places: Places, now: Date): string[] {
  const found: Quest[] = [];
  for (const id of names(places.quests)) {
    const readme = text(join(places.quests, id, "README.md"), README_BYTES);
    const front = readme?.match(/^---\n([\s\S]*?)\n---/)?.[1];
    if (!readme || !front) continue;
    const status = field(front, "status");
    const priority = field(front, "priority");
    if (!(status === "active" || status === "blocked") || !(priority === "driving" || priority === "active")) continue;
    const day = Date.parse(field(front, "updated") ?? "");
    if (!(now.getTime() - day < QUIET_DAYS * DAY_MS)) continue;
    const title = readme.match(/^# +(.+)$/m)?.[1]?.trim();
    const name = field(front, "short") ?? title ?? id;
    const journey = text(join(places.quests, id, "JOURNEY.md"), JOURNEY_BYTES);
    const newest = journey?.match(/^- \*\*[^*]+\*\*: *([\s\S]+?)(?=\n- \*\*|\n\n|(?![\s\S]))/m)?.[1];
    let updated = 0;
    try {
      updated = statSync(join(places.quests, id, "README.md")).mtimeMs;
    } catch {
      // Read a moment ago; a race with a rewrite only costs its place.
    }
    const named = title && title !== name ? `${name}, ${clip(title, 60)}` : name;
    found.push({
      line: `- ${id.slice(-6)} ${named} (${status}${priority === "driving" ? ", driving" : ""})${newest ? `: ${clip(newest, JOURNEY_CHARS)}` : ""}`,
      updated,
    });
  }
  return found
    .sort((a, b) => b.updated - a.updated)
    .slice(0, MOST_QUESTS)
    .map((quest) => quest.line);
}

/** The digest as it reads now. */
export function digest(now: Date, places: Places, person?: string): string {
  const when = now.toLocaleString("en-CA", {
    weekday: "long",
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    timeZoneName: "short",
  });
  const machine = [power(places), disk(places) && `disk: ${disk(places)}`, network(places) && `network: ${network(places)}`]
    .filter(Boolean)
    .join("; ");
  const lines = [
    "What's true right now, read from this machine a moment ago. Answer from it when it holds the answer; when it doesn't, say so in a sentence rather than guessing.",
    ...(person ? [`Asking: ${person}, whose machine this is.`] : []),
    `Now: ${when}.`,
    `Machine: ${hostname()}${machine ? `; ${machine}` : ""}.`,
  ];
  const waits = waiting(places);
  lines.push(waits.length ? "Waiting on them:" : "Nothing waits on them.", ...waits);
  const worked = quests(places, now);
  if (worked.length) lines.push("Quests the agents are working on (id, name, status: newest step):", ...worked);
  return lines.join("\n");
}

/** Puts the digest after the system prompt as each question starts. */
export default function (pi: ExtensionAPI) {
  pi.on("before_agent_start", (event) => ({
    systemPrompt: `${event.systemPrompt}\n\n${digest(new Date(), placesHere(), process.env.ASK_PERSON || undefined)}`,
  }));
}
