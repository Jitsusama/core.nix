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
import {
  closeSync,
  openSync,
  readdirSync,
  readFileSync,
  readSync,
  statfsSync,
  statSync,
} from "node:fs";
import { homedir, hostname } from "node:os";
import { join } from "node:path";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

/** The most quests listed, the most recently updated kept. */
const MOST_QUESTS = 30;
/** How long since a quest was last updated it's still listed. */
const QUIET_DAYS = 7;
const SECOND_MS = 1000;
const DAY_MS = 24 * 60 * 60 * SECOND_MS;
/** How much of a quest's newest Journey line is kept. */
const JOURNEY_CHARS = 140;
/** How much of a waiting item's title is kept. */
const WAIT_CHARS = 160;
/** How much of a quest's title is kept beside its short name. */
const TITLE_CHARS = 60;
/** How much of a quest's id names it: the six letters after its date. */
const SHORT_ID = 6;
/** How much of a README is read for its front matter and title. */
const README_BYTES = 16_384;
/** How much of a Journey is read for its newest line. */
const JOURNEY_BYTES = 4096;
const KIB = 1024;
const GIB = KIB * KIB * KIB;

/** The quests listed: those being worked, and driven or active. */
const WORKED = new Set(["active", "blocked"]);
const DRIVEN = new Set(["driving", "active"]);

const FRONT_MATTER = /^---\n([\s\S]*?)\n---/;
const TITLE = /^# +(.+)$/m;
/** A Journey's first entry, through its last line, however many it spans. */
const NEWEST_ENTRY = /^- \*\*[^*]+\*\*: *([\s\S]+?)(?=\n- \*\*|\n\n|(?![\s\S]))/m;

interface Quest {
  line: string;
  updated: number;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** A file's text, or undefined when there's none to read. */
function text(path: string, bytes?: number): string | undefined {
  try {
    if (bytes === undefined) {
      return readFileSync(path, "utf8");
    }
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
  if (read === undefined) {
    return undefined;
  }
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

/** One of a power supply's facts, as Linux gives it. */
function supplySays(supplies: string, supply: string, fact: string): string | undefined {
  return text(join(supplies, supply, fact))?.trim();
}

/** A battery's charge and what it's doing, when it says its charge. */
function battery(supplies: string, supply: string): string | undefined {
  const capacity = supplySays(supplies, supply, "capacity");
  if (!capacity) {
    return undefined;
  }
  const status = supplySays(supplies, supply, "status")?.toLowerCase();
  return `battery ${capacity}%${status ? `, ${status}` : ""}`;
}

/** The battery and whether it's on power, from Linux's power supplies. */
function power(places: Places): string | undefined {
  const supplies = join(places.sys, "power_supply");
  const kinds = names(supplies).map((supply) => ({
    supply,
    kind: supplySays(supplies, supply, "type"),
  }));
  const said = kinds
    .filter(({ kind }) => kind === "Battery")
    .map(({ supply }) => battery(supplies, supply))
    .filter((line): line is string => line !== undefined);
  // Linux calls a charger's supply Mains.
  const chargers = kinds.filter(({ kind }) => kind === "Mains");
  if (chargers.length > 0) {
    const online = chargers.some(({ supply }) => supplySays(supplies, supply, "online") === "1");
    said.push(online ? "on power" : "not on power");
  }
  return said.length > 0 ? said.join(", ") : undefined;
}

/** The disk, as the disk guard says it, else the home's free space. */
function disk(places: Places): string | undefined {
  const guard = places.runtime
    ? json(join(places.runtime, "disk-guard", "status.json"))
    : undefined;
  if (isRecord(guard) && typeof guard.says === "string") {
    return guard.says;
  }
  try {
    const stats = statfsSync(places.home);
    return `${((stats.bavail * stats.bsize) / GIB).toFixed(1)} GiB free`;
  } catch {
    // No way to ask on this machine.
    return undefined;
  }
}

/** Whether a link is Wi-Fi or wired. */
function linkKind(links: string, link: string): string {
  return names(join(links, link, "wireless")).length > 0 || link.startsWith("wl")
    ? "Wi-Fi"
    : "wired";
}

/** The physical network links that are up, Wi-Fi or wired. */
function network(places: Places): string | undefined {
  const links = join(places.sys, "net");
  const all = names(links);
  const up = all.filter(
    (link) =>
      text(join(links, link, "operstate"))?.trim() === "up" &&
      names(join(links, link, "device")).length > 0,
  );
  if (up.length === 0) {
    return all.length > 0 ? "no physical link up" : undefined;
  }
  return up.map((link) => `${link} (${linkKind(links, link)})`).join(", ");
}

/** A waiting item with a title, the only kind said. */
function isTitled(item: unknown): item is Record<string, unknown> & { title: string } {
  return isRecord(item) && typeof item.title === "string";
}

/** What one waiting file says waits, each item on a line. */
function waitsIn(file: string): string[] {
  const read = json(file);
  if (isRecord(read) && Array.isArray(read.items)) {
    const from = typeof read.quest === "string" ? ` (${read.quest.slice(-SHORT_ID)})` : "";
    return read.items.filter(isTitled).map((item) => {
      const when = typeof item.when === "string" ? `, ${item.when}` : "";
      return `- ${clip(item.title, WAIT_CHARS)}${from}${when}`;
    });
  }
  return [];
}

/** Each thing waiting on the person: the orchestrator's, then each quest's. */
function waiting(places: Places): string[] {
  const mission = join(places.state, "mission");
  const each = names(join(mission, "waiting")).map((name) => join(mission, "waiting", name));
  return [join(mission, "queue.json"), ...each].flatMap(waitsIn);
}

function field(front: string, name: string): string | undefined {
  return new RegExp(`^${name}: *(.+)$`, "m").exec(front)?.[1]?.trim();
}

/** Whether a quest is being driven or worked, and was updated lately. */
function isWorked(front: string, now: Date): boolean {
  const day = Date.parse(field(front, "updated") ?? "");
  return (
    WORKED.has(field(front, "status") ?? "") &&
    DRIVEN.has(field(front, "priority") ?? "") &&
    now.getTime() - day < QUIET_DAYS * DAY_MS
  );
}

/** When a quest's README last changed, or never, should it go meanwhile. */
function updatedAt(places: Places, id: string): number {
  try {
    return statSync(join(places.quests, id, "README.md")).mtimeMs;
  } catch {
    // Read a moment ago; a race with a rewrite only costs its place.
    return 0;
  }
}

/** A quest's line: its id, its names, its status and its newest step. */
function questLine(places: Places, id: string, readme: string, front: string): string {
  const status = field(front, "status");
  const title = readme.match(TITLE)?.[1]?.trim();
  const name = field(front, "short") ?? title ?? id;
  const journey = text(join(places.quests, id, "JOURNEY.md"), JOURNEY_BYTES);
  const newest = journey?.match(NEWEST_ENTRY)?.[1];
  const named = title && title !== name ? `${name}, ${clip(title, TITLE_CHARS)}` : name;
  const driving = field(front, "priority") === "driving" ? ", driving" : "";
  const step = newest ? `: ${clip(newest, JOURNEY_CHARS)}` : "";
  return `- ${id.slice(-SHORT_ID)} ${named} (${status}${driving})${step}`;
}

/** A quest, when it's one being driven or worked lately. */
function questOf(places: Places, id: string, now: Date): Quest | undefined {
  const readme = text(join(places.quests, id, "README.md"), README_BYTES);
  const front = readme?.match(FRONT_MATTER)?.[1];
  if (readme && front && isWorked(front, now)) {
    return { line: questLine(places, id, readme, front), updated: updatedAt(places, id) };
  }
  return undefined;
}

/** The quests being driven or worked lately, newest first, one line each. */
function quests(places: Places, now: Date): string[] {
  return names(places.quests)
    .map((id) => questOf(places, id, now))
    .filter((quest): quest is Quest => quest !== undefined)
    .sort((a, b) => b.updated - a.updated)
    .slice(0, MOST_QUESTS)
    .map((quest) => quest.line);
}

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
  const space = disk(places);
  const links = network(places);
  const machine = [power(places), space && `disk: ${space}`, links && `network: ${links}`]
    .filter(Boolean)
    .join("; ");
  const lines = [
    "What's true right now, read from this machine a moment ago. Answer from it when it holds " +
      "the answer; when it doesn't, say so in a sentence rather than guessing.",
    ...(person ? [`Asking: ${person}, whose machine this is.`] : []),
    `Now: ${when}.`,
    `Machine: ${hostname()}${machine ? `; ${machine}` : ""}.`,
  ];
  const waits = waiting(places);
  lines.push(waits.length > 0 ? "Waiting on them:" : "Nothing waits on them.", ...waits);
  const worked = quests(places, now);
  if (worked.length > 0) {
    lines.push("Quests the agents are working on (id, name, status: newest step):", ...worked);
  }
  return lines.join("\n");
}

/** Puts the digest after the system prompt as each question starts. */
export default function (pi: ExtensionAPI) {
  pi.on("before_agent_start", (event) => ({
    systemPrompt: `${event.systemPrompt}\n\n${digest(new Date(), placesHere(), process.env.ASK_PERSON || undefined)}`,
  }));
}
