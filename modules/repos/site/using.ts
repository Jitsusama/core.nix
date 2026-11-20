// What works in a repository and what it takes, as the repos daemon reads it from /proc each
// minute (src/using.rs): the processes whose working folder lies in each checkout or tree,
// their share of a core between two readings, and the memory they hold, each one's
// proportional share. Only Joel's own processes are read, so a build the Nix daemon runs for
// one is counted nowhere, and only Linux has /proc. Nothing here touches the page, so each
// reading is tried in Node (tests/using.test.ts).

import { type Fields, isObject, number, text } from "../../local-sites/kit/records.ts";
import { bytes } from "./weight.ts";

/** What works in one folder, or a repository's folders together. */
interface Use {
  readonly processes: number;
  /** The memory they hold, in bytes. */
  readonly memory: number;
  /** Their processor time as a share of one core, unknown until two readings are taken. */
  readonly cpu: number | undefined;
}

/** A repository's use, whole and by folder. */
interface Using {
  readonly repository: string;
  readonly whole: Use;
  readonly folders: ReadonlyMap<string, Use>;
}

/** A use read from `fields`, a record's or one folder's in it. */
function readUse(fields: Fields): Use {
  return {
    processes: number(fields, "processes") ?? 0,
    memory: number(fields, "memory") ?? 0,
    cpu: number(fields, "cpu"),
  };
}

/** A `using:` record read, its folders by path. */
function usingOf(record: Fields): Using {
  const told = record["folders"];
  const folders = (Array.isArray(told) ? told : []).flatMap((each: unknown) => {
    const path = isObject(each) ? text(each, "path") : undefined;
    return isObject(each) && path !== undefined ? [[path, readUse(each)] as const] : [];
  });
  return {
    repository: text(record, "repository") ?? "",
    whole: readUse(record),
    folders: new Map(folders),
  };
}

// Under this share, a core's use says only that it's little, since a hundredth of a minute
// is a blink.
const LEAST = 0.01;

/** A share of one core as it's said: a percentage of a core, or cores from one whole. */
function share(cpu: number): string {
  if (cpu < LEAST) {
    return cpu === 0 ? "0% of a core" : "under 1% of a core";
  }
  if (cpu < 1) {
    return `${Math.round(cpu * 100)}% of a core`;
  }
  return `${cpu.toFixed(1)} cores`;
}

/** A use in a row's few words: its processor and memory, and nothing when nothing works in
 * it, so a quiet repository's row stays quiet. */
function usedBriefly(use: Use | undefined): string {
  if (use === undefined || use.processes === 0) {
    return "";
  }
  const memory = `${bytes(use.memory)} of memory`;
  return use.cpu === undefined ? memory : `${share(use.cpu)}, ${memory}`;
}

/** A use as the drawer says it: how many processes work there, then what they take. */
function usedFully(use: Use | undefined): string {
  if (use === undefined || use.processes === 0) {
    return "";
  }
  const count = use.processes.toLocaleString("en-CA");
  const processes = `${count} ${use.processes === 1 ? "process" : "processes"}`;
  return `${processes}: ${usedBriefly(use)}`;
}

export type { Use, Using };
export { usedBriefly, usedFully, usingOf };
