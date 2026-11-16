// A machine mycelium runs on, in its device's panel, as mycelium sees it:
// what runs there, what it has left and what it's short of, with a way to
// its place on the mycelium map. mycelium names a machine by its host name,
// as net does; what counts as short is mycelium's to say, not the page's.

import { type Child, facts, glyph, h } from "./elements.ts";
import type { Device } from "./model.ts";
import { homeOf, type Machine, type Mycelium } from "./mycelium.ts";

/** Each standing mycelium gives a cell, in the words the panel says it in. */
const STANDINGS: readonly (readonly [string, string])[] = [
  ["needs-you", "waiting on you"],
  ["moving", "moving"],
  ["quiet", "quiet"],
  ["dormant", "dormant"],
];

const GIGA = 1e9;
/** From a hundred gigabytes on, a tenth of one says nothing more. */
const WHOLE_GIGABYTES = 100 * GIGA;

const gb = (bytes: number): string => (bytes / GIGA).toFixed(bytes >= WHOLE_GIGABYTES ? 0 : 1);

/** `left` of `whole` gigabytes, when mycelium gives both. */
function free(left: number | undefined, whole: number | undefined): string | null {
  return left === undefined || whole === undefined ? null : `${gb(left)} of ${gb(whole)} GB free`;
}

/** Its cells, how many in all and how many stand each way. */
function cells(machine: Machine): string {
  const count = [...machine.cells.values()].reduce((sum, n) => sum + n, 0);
  const standing = STANDINGS.flatMap(([word, words]) => {
    const n = machine.cells.get(word) ?? 0;
    return n > 0 ? [`${n} ${words}`] : [];
  });
  return count > 0 ? `${count}: ${standing.join(", ")}` : "none";
}

/** What work waits on, as mycelium measures it. */
function waiting(machine: Machine): string | null {
  const p = machine.pressure;
  if (p === undefined) {
    return null;
  }
  const each = [
    `${Math.round(p.cpu)}% for the processor`,
    `${Math.round(p.memory)}% for memory`,
    `${Math.round(p.io)}% for the disk`,
  ];
  return each.join(", ");
}

/** The machine `device` is, as mycelium sees it, if mycelium runs on it. */
function inMycelium(mycelium: Mycelium, device: Device): Child[] {
  const machine = mycelium.machines.find((each) => each.name === device.name);
  if (machine === undefined) {
    return [];
  }
  // Gold, with the diamond, so what's short is never told by colour alone.
  const short =
    machine.over.length > 0
      ? h("span", { class: "look" }, glyph(true), machine.over.join(" and "))
      : null;
  const home = `../${homeOf(machine.name)}`;
  return [
    h("h3", {}, "In Mycelium"),
    facts([
      ["cells", cells(machine)],
      ["waiting", waiting(machine)],
      ["memory", free(machine.available, machine.memory), true],
      ["disk", free(machine.diskFree, machine.disk), true],
      ["short of", short],
    ]),
    h(
      "p",
      { class: "hint" },
      "Waiting is the share of the last minute work spent held up for each. ",
      "A cell is one session of agents' work.",
    ),
    h("p", {}, h("a", { href: home }, `${machine.name} on the mycelium map`)),
  ];
}

export { inMycelium };
