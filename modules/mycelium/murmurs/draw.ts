// The murmurs page's markup: the flock at dusk over its wire, and a murmur's
// row. Each murmur still in the air is a bird in the flock of the cell that
// said it, and a hairline runs from it to the flock of each quest it is tied
// to whose cell murmurs here too; a settled murmur has come down to roost on the wire. A bird is told
// from a roosting one by its shape, a stroke in flight and a body at rest,
// never by colour alone. Every word read from a record is escaped before it
// reaches the page.

import { ago, type Headline, type Murmur, questsOf } from "./model.ts";

const ESCAPES: { readonly [character: string]: string } = {
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  '"': "&quot;",
  "'": "&#39;",
};

/** The sky's measures, in its own units. */
const SKY = {
  width: 480,
  height: 150,
  wire: 128,
  /** How far a flock spreads from its middle, across and up and down. */
  spread: 34,
  rise: 22,
  /** The most birds drawn; the list holds every murmur the node keeps. */
  most: 64,
  /** How much larger than its own units the sky is drawn, where the page has room. */
  scale: 1.5,
} as const;

/** A whole number from `words`, the same for the same words, to place a bird. */
function seeded(words: string): number {
  let seed = 0;
  for (const character of words) {
    seed = (seed * 31 + (character.codePointAt(0) ?? 0)) % 2_147_483_647;
  }
  return seed;
}

/** A number in [-1, 1) drawn from `seed` by its `nth` turn. */
function scatter(seed: number, nth: number): number {
  return ((seed * (nth + 7)) % 1000) / 500 - 1;
}

/** `text` safe to put in markup, as text or an attribute's value. */
function escaped(text: string): string {
  return text.replace(/[&<>"']/gu, (character) => ESCAPES[character] ?? character);
}

/** The headline's markup: its count links to every murmur, and a part still in the air to those. */
function headlineOf(whole: Headline): string {
  const lead = escaped(whole.lead);
  const turn = escaped(whole.turn);
  if (whole.said === 0) {
    return `${lead} <em>${turn}</em>`;
  }
  const all = `<a href="#all" data-show="all">${lead}</a>`;
  if (whole.open === 0 || whole.open === whole.said) {
    return `${all} <em>${turn}</em>`;
  }
  return `${all} <em><a href="#air" data-show="air">${turn}</a></em>`;
}

/** Where each quest's flock gathers: spread across the sky in turn, rising and falling. */
function flocks(quests: readonly string[]): ReadonlyMap<string, { x: number; y: number }> {
  const room = SKY.width / Math.max(1, quests.length);
  return new Map(
    quests.map((quest, at) => [
      quest,
      { x: room * (at + 0.5), y: 48 + (at % 2 === 0 ? -8 : 14) + scatter(seeded(quest), 1) * 6 },
    ]),
  );
}

/** A bird in flight at `x`, `y`: two wing strokes meeting at its body. */
function bird(murmur: Murmur, x: number, y: number, on: boolean): string {
  const wing = 3.5 + (seeded(murmur.key) % 3) * 0.6;
  return `<path class="bird${on ? " on" : ""}" data-key="${escaped(murmur.key)}" d="M${(x - wing).toFixed(1)} ${(y - 2).toFixed(1)}Q${(x - wing / 2).toFixed(1)} ${(y - 3.5).toFixed(1)} ${x.toFixed(1)} ${y.toFixed(1)}Q${(x + wing / 2).toFixed(1)} ${(y - 3.5).toFixed(1)} ${(x + wing).toFixed(1)} ${(y - 2).toFixed(1)}"/>`;
}

/** A bird come to roost on the wire at `x`. */
function roosting(murmur: Murmur, x: number, on: boolean): string {
  return `<ellipse class="roost${on ? " on" : ""}" data-key="${escaped(murmur.key)}" cx="${x.toFixed(1)}" cy="${SKY.wire - 3}" rx="2.6" ry="3.2"/>`;
}

/** The sky over the wire, with a bird for each murmur and a roost for each settled. */
function skyOf(murmurs: readonly Murmur[], cursor: string | null): string {
  const drawn = murmurs.slice(0, SKY.most);
  const gathered = flocks(questsOf(drawn));
  const air = drawn.filter((each) => each.settled === null);
  const roosts = drawn.filter((each) => each.settled !== null);
  const places = air.map((murmur) => {
    const home = gathered.get(murmur.writer) ?? { x: SKY.width / 2, y: 48 };
    const seed = seeded(murmur.key);
    return {
      murmur,
      x: home.x + scatter(seed, 2) * SKY.spread,
      y: home.y + scatter(seed, 3) * SKY.rise,
    };
  });
  const ties = places
    .flatMap(({ murmur, x, y }) =>
      murmur.ties.flatMap((quest) => {
        const there = gathered.get(quest);
        return there === undefined
          ? []
          : [
              `<path class="tie" d="M${x.toFixed(1)} ${y.toFixed(1)}L${there.x.toFixed(1)} ${there.y.toFixed(1)}"/>`,
            ];
      }),
    )
    .join("");
  const birds = places
    .map(({ murmur, x, y }) => bird(murmur, x, y, murmur.key === cursor))
    .join("");
  const room = SKY.width / (roosts.length + 1);
  const rest = roosts
    .map((murmur, at) => roosting(murmur, room * (at + 1), murmur.key === cursor))
    .join("");
  const more =
    murmurs.length > drawn.length
      ? `<text class="note" x="${SKY.width - 8}" y="${SKY.height - 4}" text-anchor="end">and ${murmurs.length - drawn.length} more in the list</text>`
      : "";
  return `<svg viewBox="0 0 ${SKY.width} ${SKY.height}" style="--w:${SKY.width * SKY.scale}px" aria-hidden="true">
<path class="wire" d="M0 ${SKY.wire}C${SKY.width / 3} ${SKY.wire + 6} ${(SKY.width * 2) / 3} ${SKY.wire + 6} ${SKY.width} ${SKY.wire}"/>
${ties}${birds}${rest}${more}
</svg>`;
}

/** The quests `murmur` is tied to, said as a list. */
function tiedTo(murmur: Murmur): string {
  if (murmur.ties.length === 0) {
    return "";
  }
  return ` · tied to ${murmur.ties.map((quest) => `<code>${escaped(quest)}</code>`).join(", ")}`;
}

/** The row of `murmur`, its mark in the gutter; `now` is in seconds. */
function murmurRow(murmur: Murmur, now: number): string {
  const mark =
    murmur.settled === null
      ? `<svg class="mm" viewBox="0 0 16 10" aria-hidden="true"><path class="bird" d="M2 4Q5 1 8 6Q11 1 14 4"/></svg>`
      : `<svg class="mm" viewBox="0 0 16 10" aria-hidden="true"><path class="wire" d="M0 9H16"/><ellipse class="roost" cx="8" cy="5.5" rx="2.6" ry="3.2"/></svg>`;
  const settled =
    murmur.settled === null
      ? ""
      : `<span class="settled">Settled: ${escaped(murmur.settled)}</span>`;
  return `<div role="option" id="murmur-${escaped(murmur.key)}" data-key="${escaped(murmur.key)}" aria-selected="false">
  <span class="mk">${mark}</span>
  <span class="line">
    <span class="words" dir="auto">${escaped(murmur.words)}</span>
    <span class="said">from <bdi>${escaped(murmur.quest)}</bdi> <code>${escaped(murmur.writer)}</code> · ${escaped(ago(murmur.at, now))}${tiedTo(murmur)}</span>
    ${settled}
  </span>
</div>`;
}

export { escaped, headlineOf, murmurRow, skyOf };
