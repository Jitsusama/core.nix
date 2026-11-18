// What ties a machine to the other three parts, as its record carries them
// (modules/system/src/parts/ties.rs): the mycelium's cells on it, where it
// meets the network, the code it holds, its own figures and what Joel can
// do with it. Read leniently, each tie none that the record doesn't say,
// so a machine from a daemon before ties draws with each tie not read,
// and no act. How each is said is machine.ts's; this only reads.

import {
  type Fields,
  isObject,
  number,
  object,
  text,
  texts,
} from "../../local-sites/kit/records.ts";

const EMPTY: Fields = {};

/** The mycelium's cells on a machine. */
interface Cells {
  readonly busy: number;
  readonly idle: number;
  readonly ended: number;
}

/** Where a machine meets the network: one device the network names for it. */
interface Place {
  readonly device: string | undefined;
  /** Its band, or "wired". */
  readonly by: string | undefined;
  readonly point: string | undefined;
  readonly bars: number | undefined;
  readonly room: number | undefined;
}

/** The code a machine holds, counted. */
interface Code {
  readonly repositories: number;
  readonly checkouts: number;
  readonly trees: number;
  readonly behind: number;
  readonly changes: number;
}

/** A machine's own figures, as its node says them. */
interface Figures {
  readonly model: string | undefined;
  readonly system: string | undefined;
  readonly disk: number | undefined;
  readonly diskFree: number | undefined;
  readonly cpuBusy: number | undefined;
  readonly memory: number | undefined;
  readonly memoryAvailable: number | undefined;
  readonly charge: number | undefined;
  readonly onBattery: boolean | undefined;
  readonly updatedAt: string | undefined;
  readonly updateStep: string | undefined;
}

/** What Joel can do with a machine: the launcher's link and the commands. */
interface Acts {
  readonly link: string | undefined;
  readonly commands: readonly string[];
}

/** What ties a machine to the other parts; each tie none when it wasn't read. */
interface Ties {
  readonly cells: Cells | undefined;
  readonly network: readonly Place[] | undefined;
  readonly code: Code | undefined;
  readonly figures: Figures;
  readonly acts: Acts;
}

/** A count the record says, or none when it says none or a negative. */
function counted(record: Fields, field: string): number | undefined {
  const value = number(record, field);
  return value === undefined || value < 0 ? undefined : value;
}

/** A yes or no the record says, or none. */
function flag(record: Fields, field: string): boolean | undefined {
  const value = record[field];
  return typeof value === "boolean" ? value : undefined;
}

function cellsOf(said: Fields | undefined): Cells | undefined {
  if (said === undefined) {
    return undefined;
  }
  return {
    busy: counted(said, "busy") ?? 0,
    idle: counted(said, "idle") ?? 0,
    ended: counted(said, "ended") ?? 0,
  };
}

function placesOf(said: unknown): readonly Place[] | undefined {
  if (!Array.isArray(said)) {
    return undefined;
  }
  return said.filter(isObject).map((place) => ({
    device: text(place, "device"),
    by: text(place, "by"),
    point: text(place, "point"),
    bars: counted(place, "bars"),
    room: counted(place, "room_mbps"),
  }));
}

function codeOf(said: Fields | undefined): Code | undefined {
  if (said === undefined) {
    return undefined;
  }
  const of = (field: string): number => counted(said, field) ?? 0;
  return {
    repositories: of("repositories"),
    checkouts: of("checkouts"),
    trees: of("trees"),
    behind: of("behind"),
    changes: of("changes"),
  };
}

function figuresOf(said: Fields): Figures {
  return {
    model: text(said, "model"),
    system: text(said, "system"),
    disk: counted(said, "disk"),
    diskFree: counted(said, "disk_free"),
    cpuBusy: counted(said, "cpu_busy"),
    memory: counted(said, "memory"),
    memoryAvailable: counted(said, "memory_available"),
    charge: counted(said, "charge"),
    onBattery: flag(said, "on_battery"),
    updatedAt: text(said, "updated_at"),
    updateStep: text(said, "update_step"),
  };
}

/** A machine's ties as its record says them, each none that the record doesn't say. */
function tiesOf(value: unknown): Ties {
  const said = isObject(value) ? value : EMPTY;
  const acts = object(said, "acts") ?? EMPTY;
  return {
    cells: cellsOf(object(said, "cells")),
    network: placesOf(said["network"]),
    code: codeOf(object(said, "code")),
    figures: figuresOf(object(said, "figures") ?? EMPTY),
    acts: { link: launcherLink(text(acts, "link")), commands: texts(acts, "commands") ?? [] },
  };
}

/** The scheme the launcher answers, the only one a machine's act may go by. */
const LAUNCHER = "mycelium:";

/** `link`, when it goes to the launcher; none otherwise, so a record can't send Joel elsewhere. */
function launcherLink(link: string | undefined): string | undefined {
  return link?.startsWith(LAUNCHER) === true ? link : undefined;
}

export type { Acts, Cells, Code, Figures, Place, Ties };
export { tiesOf };
