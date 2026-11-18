// The system page's markup, machine by machine. A machine is the bridge
// between the mycelium, the network and the code (Joel, 2026-11-17), so
// each has a section of its own: its name and how it stands, then its four
// ties, each said in words with the way to see it whole, then what can be
// done from here. The other three parts follow, said across the machines.
// How a thing stands is its health in the system's own words, good, watch,
// bad or not read, each with its own mark so none is told by colour. Every
// row the keys step through is a link marked data-row, in reading order,
// so the cursor is focus itself and Return follows it. Every word read
// from a record is escaped before it reaches the page.

import {
  cellsSaid,
  codeSaid,
  itselfSaid,
  kindOf,
  type MachineHeadline,
  networkSaid,
  type Said,
  standing,
} from "./machine.ts";
import { capital, type Health, named, type Part, type System, type Thing, WORDS } from "./model.ts";

/** The most machines drawn whole, worst first, so thousands stay short. */
const MACHINES_DRAWN = 8;

/** The other parts, said across the machines, in the order they are drawn. */
const ACROSS = ["mycelium", "network", "code"] as const;

/** What each other part's link says, to see it whole. */
const WHOLE: { readonly [part: string]: string } = {
  mycelium: "See the whole mycelium",
  network: "See the whole network",
  code: "See all your code",
};

const ESCAPES: { readonly [character: string]: string } = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
  "'": "&#39;",
};

/** One tie of a machine: its name, what it says, and where to see it whole. */
interface Tie {
  readonly name: string;
  readonly said: Said;
  readonly link?: { readonly href: string; readonly words: string };
}

/** `text` safe to put in markup, as text or an attribute's value. */
function escaped(text: string): string {
  return text.replace(/[&<>"']/gu, (character) => ESCAPES[character] ?? character);
}

/** The mark `health` is drawn with, its word said only to a screen reader. */
function mark(health: Health): string {
  return `<span class="mark ${health}" aria-hidden="true"></span>`;
}

/** How `health` is said to a screen reader, after a name. */
function spoken(health: Health): string {
  // In parentheses, since a reader takes the hidden span out of flow as a
  // block and puts a space before it: "osprey (watch)" reads right where
  // "osprey , watch" would not.
  return `<span class="sr"> (${WORDS[health].toLowerCase()})</span>`;
}

/** Where a thing's link goes: its home's address, or the mycelium site. */
function home(thing: Thing): string {
  return `../${thing.address ?? "mycelium/"}`;
}

/** The machines drawn whole, worst first as the machines part lists them. */
function machinesOf(system: System): readonly Thing[] {
  return system.parts.find((part) => part.part === "machines")?.things ?? [];
}

/** The headline: how many machines, then how they stand in their worst health's colour. */
function headlineOf(heading: MachineHeadline): string {
  return `${escaped(heading.machines)}, <em class="health ${heading.health}">${escaped(heading.stand)}.</em>`;
}

/** The places at the top: the machines drawn whole, then the parts across them. */
function placesOf(system: System): string {
  const machines = machinesOf(system)
    .slice(0, MACHINES_DRAWN)
    .map(
      (thing, at) =>
        `<a class="place" href="#machine-${at + 1}" data-place="machine-${at + 1}" translate="no">${mark(thing.health)}${escaped(thing.name)}${spoken(thing.health)}</a>`,
    );
  return [...machines, '<a class="place" href="#across" data-place="across">Across them</a>'].join(
    "",
  );
}

/** A command and the button that copies it, which y presses for its section's first. */
function command(said: string): string {
  const words = escaped(said);
  return `<span class="command"><code>${words}</code><button type="button" class="copy" data-command="${words}" aria-label="Copy ${words}">copy</button></span>`;
}

/** One tie, drawn: its name, what it says, what is aside, and the way to see it whole. */
function tieOf(tie: Tie, key: string): string {
  const link =
    tie.link === undefined
      ? ""
      : `<a class="row deeper" data-row="${escaped(key)}" href="${escaped(tie.link.href)}">${escaped(tie.link.words)}</a>`;
  return `<section class="tie"><h3>${escaped(tie.name)}</h3><p class="said">${escaped(tie.said.said)}</p><p class="aside">${escaped(tie.said.aside)}</p>${link}</section>`;
}

/** A machine's four ties: its cells, the network, its code and itself. */
function tiesOf(thing: Thing, when: (at: string) => string): readonly Tie[] {
  const network = networkSaid(thing.ties);
  const code = codeSaid(thing.ties, thing.name);
  return [
    {
      name: "Cells",
      said: cellsSaid(thing.ties),
      link: { href: home(thing), words: "See them on the map" },
    },
    {
      name: "Network",
      said: network,
      ...(network.device === undefined
        ? {}
        : {
            link: {
              href: `../network/#device=${encodeURIComponent(network.device)}`,
              words: "See it on the network",
            },
          }),
    },
    {
      name: "Code",
      said: code,
      ...(code.read ? { link: { href: "../code/", words: "See it in your code" } } : {}),
    },
    { name: "Itself", said: itselfSaid(thing.ties, when) },
  ];
}

/** What can be done to a machine from here: open it in the launcher, or a command. */
function actsOf(thing: Thing): string {
  const { link, commands } = thing.ties.acts;
  const open =
    link === undefined
      ? ""
      : `<a class="row act" data-row="act:${escaped(thing.name)}" href="${escaped(link)}">Open in the Launcher</a>`;
  const said = commands.map(command).join("");
  return open === "" && said === "" ? "" : `<p class="acts">${open}${said}</p>`;
}

/** One machine, drawn whole: its name, kind and standing, its ties and its acts. */
function machineOf(thing: Thing, at: number, here: string, when: (at: string) => string): string {
  const id = `machine-${at + 1}`;
  const name = escaped(thing.name);
  const kind = kindOf(thing, here);
  const kindLine = kind === undefined ? "" : `<p class="kind">${escaped(kind)}</p>`;
  const ties = tiesOf(thing, when)
    .map((tie) => tieOf(tie, `${tie.name.toLowerCase()}:${thing.name}`))
    .join("");
  return `<section class="machine ${thing.health}" id="${id}" data-part="${id}" aria-labelledby="name-${id}">
  <h2 class="name" id="name-${id}"><a class="row" data-row="machine:${name}" data-health="${thing.health}" href="${escaped(home(thing))}"><span translate="no">${name}</span>${spoken(thing.health)}</a></h2>
  ${kindLine}
  <p class="standing health ${thing.health}">${mark(thing.health)}${escaped(standing(thing))}</p>
  <div class="ties">${ties}</div>
  ${actsOf(thing)}
</section>`;
}

/** What one other part says across the machines: what is worth a look, or what it holds. */
function partSaid(part: Part): Said {
  if (part.health === "unread") {
    return { said: WORDS.unread, aside: capital(part.says) };
  }
  const [first, ...rest] = part.side;
  if (part.health === "good" || first === undefined) {
    return { said: capital(part.holds ?? part.says), aside: "Nothing in it is worth a look." };
  }
  return {
    said: capital(first),
    aside: capital(rest.length === 0 ? (part.holds ?? "") : rest.join("; ")),
  };
}

/** One other part, across the machines: how it stands, the way to see it whole, its command. */
function acrossOf(part: Part): string {
  const said = partSaid(part);
  const name = escaped(named(part.part));
  return `<section class="tie part ${part.health}" data-part="part-${escaped(part.part)}"><h3>${mark(part.health)}${name}${spoken(part.health)}</h3><p class="said">${escaped(said.said)}</p><p class="aside">${escaped(said.aside)}</p><a class="row deeper" data-row="part:${escaped(part.part)}" data-health="${part.health}" href="../${escaped(part.deeper.site)}">${escaped(WHOLE[part.part] ?? `See all of ${named(part.part)}`)}</a>${command(part.deeper.command)}</section>`;
}

/** The machines beyond those drawn whole, said as a row to the map. */
function moreOf(system: System): string {
  const more = machinesOf(system).length - MACHINES_DRAWN;
  if (more <= 0) {
    return "";
  }
  const words = `And ${more.toLocaleString("en-CA")} more ${more === 1 ? "machine" : "machines"}, on the map`;
  return `<p class="more"><a class="row" data-row="machines:more" href="../mycelium/">${escaped(words)}</a></p>`;
}

/**
 * The machines part when it can't be drawn machine by machine: when it
 * wasn't read, or holds none, it says so in its own words.
 */
function bareOf(system: System): string {
  const part = system.parts.find((one) => one.part === "machines");
  if (part === undefined || machinesOf(system).length > 0) {
    return "";
  }
  return `<p class="bare health ${part.health}">${mark(part.health)}${escaped(capital(part.says))}</p>`;
}

/**
 * Every machine drawn whole, then the other parts across them. `here` is
 * the machine the page reads from; `when` says a time, as "at 02:19".
 */
function partsOf(system: System, when: (at: string) => string): string {
  const machines = machinesOf(system)
    .slice(0, MACHINES_DRAWN)
    .map((thing, at) => machineOf(thing, at, system.machine, when));
  const across = ACROSS.map((name) => system.parts.find((part) => part.part === name))
    .filter((part) => part !== undefined)
    .map(acrossOf)
    .join("");
  return `${bareOf(system)}${machines.join("\n")}${moreOf(system)}
<section class="across" id="across" data-part="across" aria-labelledby="name-across">
  <h2 class="name" id="name-across">Across <em>them</em></h2>
  <div class="ties">${across}</div>
  <p class="later">Not shown yet: the builders your machines lend work to.</p>
</section>`;
}

/**
 * The legend: each health's mark beside its word, said once, an item each
 * so a screen reader hears the words apart, the marks being hidden from it.
 */
function legend(): string {
  const healths: readonly Health[] = ["good", "watch", "bad", "unread"];
  return healths
    .map((health) => `<li class="key">${mark(health)}<span class="word">${WORDS[health]}</span></li>`)
    .join("");
}

export { escaped, headlineOf, home, legend, MACHINES_DRAWN, machinesOf, partsOf, placesOf };
