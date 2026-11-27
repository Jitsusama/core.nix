// The system page's markup, machine by machine. A machine is the bridge
// between the mycelium, the network and the code (Joel, 2026-11-17), so
// each has a section of its own: its name and how it stands, then its four
// ties, each said in words with the way to see it whole, then what can be
// done from here. A machine the node can't reach carries nothing to draw,
// so those out of reach are named together after the rest, in a line each,
// and never take the room of one that carries the work (Joel, 2026-11-23:
// "this view shows a shit ton of machines down, and in that state, its UX
// is rather shit"). The other three parts follow, said across the machines.
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
  worstOf,
} from "./machine.ts";
import type { Look, Looks } from "./looks.ts";
import {
  capital,
  type Health,
  named,
  numbered,
  type Part,
  type System,
  type Thing,
  WORDS,
} from "./model.ts";

/** The most machines drawn whole, worst first, so thousands stay short. */
const MACHINES_DRAWN = 8;

/** The row the machines past those drawn are reached by. */
const MORE_ROW = "machines:more";

/** The most machines out of reach named, the rest counted, so a fleet gone stays a glance. */
const AWAY_NAMED = 24;

/** The section the machines out of reach are named in. */
const AWAY = "away";

/** The row the machines out of reach past those named are reached by. */
const AWAY_MORE_ROW = `${AWAY}:more`;

/** The row a machine's name sits on, the first of its section. */
function machineRow(name: string): string {
  return `machine:${name}`;
}

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

/** This page's path from the sites' root, where a machine's home address starts. */
const HERE = "system/";

/**
 * Where a machine is on the mycelium map, its cells hanging from it. This
 * page is a machine's home, so its name and its cells lead on to the map
 * rather than back here.
 */
function onTheMap(thing: Thing): string {
  return `../mycelium/#machine=${encodeURIComponent(thing.name)}`;
}

/**
 * The section's `data-home`, when the sites say this page is the machine's
 * home: its address as the daemon read it from the rows, so a mention of
 * the machine inside its own section needn't link. A machine homed
 * elsewhere, or nowhere, claims none.
 */
function homeHere(thing: Thing): string {
  return thing.address?.startsWith(HERE) === true ? ` data-home="${escaped(thing.address)}"` : "";
}

/** The machines drawn whole, worst first as the machines part lists them. */
function machinesOf(system: System): readonly Thing[] {
  return system.parts.find((part) => part.part === "machines")?.things ?? [];
}

/** The machines in reach, worst first, which the bridge and the sections draw whole. */
function reachedOf(system: System): readonly Thing[] {
  return machinesOf(system).filter((thing) => thing.ties.reached);
}

/** The machines the node can't reach, worst first, named together after the rest. */
function awayOf(system: System): readonly Thing[] {
  return machinesOf(system).filter((thing) => !thing.ties.reached);
}

/** The headline: how many machines, then how they stand in their worst health's colour. */
function headlineOf(heading: MachineHeadline): string {
  return `${escaped(heading.machines)}, <em class="${heading.health}">${escaped(heading.stand)}.</em>`;
}

/**
 * The places at the top: the machines drawn whole, then those out of reach,
 * together, then the parts across them.
 */
function placesOf(system: System): string {
  const away = awayOf(system);
  const awayPlace =
    away.length === 0
      ? []
      : [
          `<a class="place" href="#${AWAY}" data-place="${AWAY}">${mark(worstOf(away))}Out of reach${spoken(worstOf(away))}</a>`,
        ];
  const machines = reachedOf(system)
    .slice(0, MACHINES_DRAWN)
    .map(
      (thing, at) =>
        `<a class="place" href="#machine-${at + 1}" data-place="machine-${at + 1}" translate="no">${mark(thing.health)}${escaped(thing.name)}${spoken(thing.health)}</a>`,
    );
  return [
    ...machines,
    ...awayPlace,
    '<a class="place" href="#across" data-place="across">Across them</a>',
  ].join("");
}

/** A command and the button that copies it, which y presses for its section's first. */
function command(said: string): string {
  if (said === "") {
    return "";
  }
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

/**
 * A machine's four ties: its cells, the network, its code and itself, read
 * on the machine `here` names.
 */
function tiesOf(thing: Thing, here: string, when: (at: string) => string): readonly Tie[] {
  const network = networkSaid(thing.ties);
  const code = codeSaid(thing.ties, thing.name, thing.name === here);
  return [
    {
      name: "Cells",
      said: cellsSaid(thing.ties),
      link: { href: onTheMap(thing), words: "See them on the map" },
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
  const ties = tiesOf(thing, here, when)
    .map((tie) => tieOf(tie, `${tie.name.toLowerCase()}:${thing.name}`))
    .join("");
  return `<section class="machine ${thing.health}" id="${id}" data-part="${id}"${homeHere(thing)} aria-labelledby="name-${id}">
  <h2 class="name" id="name-${id}"><a class="row" data-row="${escaped(machineRow(thing.name))}" data-health="${thing.health}" href="${escaped(onTheMap(thing))}"><span translate="no">${name}</span>${spoken(thing.health)}</a></h2>
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
  if (first === undefined) {
    return { said: capital(part.holds ?? part.says), aside: "Nothing in it is worth a look." };
  }
  // A good part has an empty side, but for one read from its owner's last
  // answer, which keeps its health and says how old it is there.
  if (part.health === "good") {
    return { said: capital(part.holds ?? part.says), aside: capital(part.side.join("; ")) };
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
  return `<section class="tie part ${part.health}" data-part="part-${escaped(part.part)}"><h3>${mark(part.health)}${name}${spoken(part.health)}</h3><p class="said">${escaped(said.said)}</p><p class="aside">${escaped(said.aside)}</p><div class="onward"><a class="row deeper" data-row="part:${escaped(part.part)}" data-health="${part.health}" href="../${escaped(part.deeper.site)}">${escaped(WHOLE[part.part] ?? `See all of ${named(part.part)}`)}</a>${command(part.deeper.command)}</div></section>`;
}

/** The machines in reach beyond those drawn whole, said as a row to the map. */
function moreOf(system: System): string {
  const more = reachedOf(system).length - MACHINES_DRAWN;
  if (more <= 0) {
    return "";
  }
  const words = `And ${more.toLocaleString("en-CA")} more ${more === 1 ? "machine" : "machines"}, on the map`;
  return `<p class="more"><a class="row" data-row="${MORE_ROW}" href="../mycelium/">${escaped(words)}</a></p>`;
}

/**
 * One machine out of reach, a line: its name, leading to the map, where its
 * last word is, when the node last heard from it, and the cells the
 * mycelium still holds on it, since work left on a machine gone quiet is
 * worth knowing. Its health is said once for them all, in the line above,
 * rather than on every row.
 */
function awayRow(thing: Thing, when: (at: string) => string): string {
  const heard =
    thing.ties.heardAt === undefined
      ? "Not heard here yet"
      : `Last heard ${when(thing.ties.heardAt)}`;
  const cells = thing.ties.cells;
  // The node counts an unreached machine's cells out of reach rather than
  // busy or idle, and a parked cell is still the mycelium's, so both count.
  // The node counts each cell in one state alone, so the sum counts none twice.
  const left =
    cells === undefined ? 0 : cells.busy + cells.idle + cells.unreachable + (cells.parked ?? 0);
  const stranded = left === 0 ? "" : `, ${counted(left, "cell")} still on it`;
  return `<li><a class="row" data-row="${escaped(machineRow(thing.name))}" data-health="${thing.health}" href="${escaped(onTheMap(thing))}"><span translate="no">${escaped(thing.name)}</span></a><span class="heard">${escaped(`${heard}${stranded}`)}</span></li>`;
}

/** `count` of `thing`, in figures, the word made plural past one. */
function counted(count: number, thing: string): string {
  return `${count.toLocaleString("en-CA")} ${count === 1 ? thing : `${thing}s`}`;
}

/**
 * The machines the node can't reach, named together after those it can: how
 * many, in one line of their worst health, then a line each, the rest past
 * what it names counted and left to the map. None draws a section of its
 * own, since it has no cells, network or figures to say but its last.
 */
function awaySection(system: System, when: (at: string) => string): string {
  const away = awayOf(system);
  if (away.length === 0) {
    return "";
  }
  const health = worstOf(away);
  const count = `${numbered(away.length)} ${away.length === 1 ? "machine" : "machines"}`;
  const stands = `${WORDS[health]}: ${count} the mycelium can't reach`;
  const rows = away
    .slice(0, AWAY_NAMED)
    .map((thing) => awayRow(thing, when))
    .join("");
  const past = away.length - AWAY_NAMED;
  const more =
    past > 0
      ? `<p class="more"><a class="row" data-row="${AWAY_MORE_ROW}" href="../mycelium/">${escaped(`And ${past.toLocaleString("en-CA")} more, on the map`)}</a></p>`
      : "";
  return `<section class="${AWAY} ${health}" id="${AWAY}" data-part="${AWAY}" aria-labelledby="name-${AWAY}">
  <h2 class="name" id="name-${AWAY}">Out of <em>reach</em></h2>
  <p class="standing health ${health}">${mark(health)}${escaped(stands)}</p>
  <ul class="gone" role="list">${rows}</ul>${more}
</section>`;
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
  // Its machines' records still to come, its own words would say them read.
  if (part.untold > 0) {
    const awaited = `${capital(numbered(part.untold))} ${part.untold === 1 ? "machine" : "machines"}`;
    return `<p class="bare health unread">${mark("unread")}${escaped(`${awaited} named, none read yet.`)}</p>`;
  }
  return `<p class="bare health ${part.health}">${mark(part.health)}${escaped(capital(part.says))}</p>`;
}

/**
 * Every machine drawn whole, then the other parts across them. `here` is
 * the machine the page reads from; `when` says a time, as "at 02:19".
 */
function partsOf(system: System, when: (at: string) => string): string {
  const machines = reachedOf(system)
    .slice(0, MACHINES_DRAWN)
    .map((thing, at) => machineOf(thing, at, system.machine, when));
  const across = ACROSS.map((name) => system.parts.find((part) => part.part === name))
    .filter((part) => part !== undefined)
    .map(acrossOf)
    .join("");
  return `${bareOf(system)}${machines.join("\n")}${moreOf(system)}${awaySection(system, when)}
<section class="across" id="across" data-part="across" aria-labelledby="name-across">
  <h2 class="name" id="name-across">Across <em>them</em></h2>
  <div class="ties">${across}</div>
  <p class="later">Not shown yet: the builders your machines lend work to.</p>
</section>`;
}

/**
 * One look, drawn as a row: its mark, whose it is, then what it is, leading
 * to where it is seen whole. Under its words, why it matters and its one
 * act, the launcher's row for its machine, each where the daemon gives it.
 * The act names its machine, so each reads alone in a list of links; it is
 * no row of the cursor's, which steps from look to look, and Tab reaches it.
 * Its words are one span, since the link is a flex box, which would make
 * each run of text an item of its own and drop the spaces between them.
 */
function lookRow(look: Look): string {
  const machine = look.machine ? ' translate="no"' : "";
  const whose = `<span class="whose"${machine}>${escaped(look.whose)}</span>`;
  const row = `<a class="row look" data-row="${escaped(look.row)}" data-health="${look.health}" href="${escaped(look.href)}">${mark(look.health)}${whose}${spoken(look.health)}<span class="words">${escaped(look.words)}</span></a>`;
  const matters = look.matters === "" ? "" : `<span>${escaped(look.matters)}</span>`;
  const name = `<span${machine}>${escaped(look.whose)}</span>`;
  const words = `<span>Open ${name} in the Launcher</span>`;
  const act =
    look.act === undefined ? "" : `<a class="launch" href="${escaped(look.act)}">${words}</a>`;
  const why = matters === "" && act === "" ? "" : `<p class="why">${matters}${act}</p>`;
  return `<li>${row}${why}</li>`;
}

/** What the list, and a, say once nothing is worth a look. */
const NOTHING = "Nothing is worth a look.";
/** What the list, and a, say while a part or record to come may hold a look. */
const NOT_KNOWN = "What is worth a look isn't known yet.";

/**
 * What is worth a look, the page's first list: a row each, those past it
 * counted, or a line saying nothing is once every part has told. Its
 * heading, in the page's markup, is for a screen reader, since the
 * headline above says it already.
 */
function looksDrawn(list: Looks): string {
  if (list.looks.length === 0) {
    // Until every part and machine has told, and every record could be read,
    // one of them may hold the look; the note says which couldn't be read.
    return list.settled
      ? `<p class="clear health good">${mark("good")}${NOTHING}</p>`
      : `<p class="more">${NOT_KNOWN}</p>`;
  }
  const more =
    list.more === 0
      ? ""
      : // Said without a place, since the rest may be machines past those drawn,
        // or a part's looks its own site holds.
        `<p class="more">${escaped(`And ${list.more.toLocaleString("en-CA")} more.`)}</p>`;
  // A list role said outright, since Safari drops an unbulleted list's,
  // and with it the count a screen reader says first.
  const rows = `<ul class="looks" role="list">${list.looks.map(lookRow).join("")}</ul>`;
  return `${rows}${more}`;
}

/**
 * The legend: each health's mark beside its word, said once, an item each
 * so a screen reader hears the words apart, the marks being hidden from it.
 */
function legend(): string {
  const healths: readonly Health[] = ["good", "watch", "bad", "unread"];
  return healths
    .map(
      (health) => `<li class="key">${mark(health)}<span class="word">${WORDS[health]}</span></li>`,
    )
    .join("");
}

export {
  AWAY,
  AWAY_MORE_ROW,
  AWAY_NAMED,
  awayOf,
  escaped,
  headlineOf,
  legend,
  looksDrawn,
  MACHINES_DRAWN,
  MORE_ROW,
  machineRow,
  machinesOf,
  NOT_KNOWN,
  NOTHING,
  partsOf,
  placesOf,
  reachedOf,
};
