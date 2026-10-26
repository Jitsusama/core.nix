/**
 * pi-ask's one tool: the launcher's own catalogue of places and actions,
 * reached as the launcher's tests reach it, through `qs ipc call launcher`
 * ($ASK_LAUNCHER is that qs). Listing reads what is there, each entry with
 * its id, the place it sits in and its words; running one does it at once
 * when it is harmless, and when it is disruptive the launcher only shows it
 * as a row ready for Return, so nothing disruptive happens from an answer.
 * Either way it lands in the launcher's own history.
 */
import { execFile } from "node:child_process";
import { Type } from "@earendil-works/pi-ai";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

/** How long the launcher has to answer before the tool gives up on it. */
const ANSWER_MS = 5_000;

/** One place or action as the launcher lists it. */
interface Entry {
  id: string;
  kind: string;
  in?: string;
  words: string;
  disruptive?: boolean;
  args?: { name: string; words?: string }[];
}

/** What the launcher says after being asked to run something. */
interface Ran {
  done?: boolean;
  offered?: boolean;
  said?: string;
  error?: string;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isEntry(value: unknown): value is Entry {
  return isRecord(value) && typeof value.id === "string" && typeof value.words === "string";
}

/** Asks the launcher, answering what it printed or why it couldn't. */
function askLauncher(args: string[], signal?: AbortSignal): Promise<string> {
  const qs = process.env.ASK_LAUNCHER || "qs";
  return new Promise((resolve, reject) => {
    execFile(
      qs,
      ["ipc", "call", "launcher", ...args],
      { timeout: ANSWER_MS, signal },
      (error, stdout, stderr) => {
        if (error) {
          const why = String(stderr).trim() || error.message;
          reject(new Error(`The launcher didn't answer: ${why}`));
        } else {
          resolve(String(stdout).trim());
        }
      },
    );
  });
}

/** One line for each entry, its id first, as the model needs to run it. */
export function listing(printed: string, match?: string): string {
  const parsed: unknown = JSON.parse(printed);
  const entries = Array.isArray(parsed) ? parsed.filter(isEntry) : [];
  const needle = match?.toLowerCase();
  const shown = needle
    ? entries.filter((entry) =>
        [entry.words, entry.in ?? "", entry.id].some((text) => text.toLowerCase().includes(needle)),
      )
    : entries;
  if (shown.length === 0) return match ? `Nothing in the launcher matches "${match}".` : "The launcher lists nothing.";
  return shown
    .map((entry) => {
      const where = entry.in ? `${entry.in} › ` : "";
      const takes = entry.args?.length ? ` (takes ${entry.args.map((arg) => arg.words ?? arg.name).join(", ")})` : "";
      const care = entry.disruptive ? " [disruptive: offered for Return, not done]" : "";
      return `${entry.id}: ${where}${entry.words}${takes}${care}`;
    })
    .join("\n");
}

/** What running an entry did, said so the answer can pass it on plainly. */
export function outcome(printed: string): string {
  const parsed: unknown = JSON.parse(printed);
  const ran: Ran = isRecord(parsed) ? parsed : {};
  if (typeof ran.error === "string") return `Not done: ${ran.error}`;
  if (ran.done) return `Done${ran.said ? `: ${ran.said}` : "."}`;
  if (ran.offered) return "Not done yet: it's disruptive, so the launcher shows it as a row and Return does it.";
  return `The launcher answered something unexpected: ${printed}`;
}

/** Registers the launcher tool. */
export default function (pi: ExtensionAPI) {
  pi.registerTool({
    name: "launcher",
    label: "Launcher",
    description:
      "The desktop launcher's places and actions: open a place or a program, change a setting such as brightness or volume, or do something to the machine. list reads them, filtered by match; run does one by its id. A disruptive one is only offered as a row for Return, so say it's ready to confirm, never that it happened.",
    promptGuidelines: [
      "When asked to open, show, change or do something on the machine, run it through the launcher tool, then say in a few words what you did.",
      "A disruptive action, such as a reboot, sleep, logging out or ending a call, is only offered as a row for Return: say it's ready to confirm, never that it happened.",
      "List with a match of a word or two from the request rather than listing everything.",
    ],
    parameters: Type.Object({
      action: Type.Union([Type.Literal("list"), Type.Literal("run")], {
        description: "list: read what the launcher offers. run: do one by its id.",
      }),
      match: Type.Optional(Type.String({ description: "For list: only entries whose words, place or id hold this." })),
      id: Type.Optional(Type.String({ description: "For run: the entry's id, as list gives it." })),
      arg: Type.Optional(Type.String({ description: "For run: the words or number an entry that takes one needs." })),
    }),
    async execute(_toolCallId, params, signal) {
      const text =
        params.action === "list"
          ? listing(await askLauncher(["list"], signal), params.match)
          : params.id
            ? outcome(await askLauncher(["run", params.id, ...(params.arg ? [params.arg] : [])], signal))
            : "Name the id to run, as list gives it.";
      return { content: [{ type: "text", text }], details: {} };
    },
  });
}
