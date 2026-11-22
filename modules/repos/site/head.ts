// The words over the grove and under the page: the headline, whose counts
// are the places, the lede saying what's worth a look and what grew, and
// the line at the foot saying how fresh it all is.

import type { Standing } from "../../local-sites/kit/feed.ts";
import { said } from "../../local-sites/kit/status.ts";
import { escaped } from "./draw.ts";
import { type Grove, growing, plural, type Repository } from "./model.ts";

/** What the head says, as markup, and the tab's title. */
interface Head {
  readonly title: string;
  readonly lede: string;
  readonly tab: string;
}

// The lede names at most this many things worth a look, so it stays a sentence or two.
const WORTH_SAID = 2;

/** A repository's name, linked to its place on this page. */
function named(repository: Repository): string {
  return `<a href="${escaped(repository.address)}"><span class="rn" translate="no">${escaped(repository.name)}</span></a>`;
}

/** The headline and its lede: how things stand, then what's worth a look. */
function headOf(grove: Grove): Head {
  const all = grove.repositories.length;
  const grew = growing(grove).length;
  const worth = grove.worth
    .slice(0, WORTH_SAID)
    .map((each) => `${named(each.repository)} ${escaped(each.says)}`);
  const busiest = [...growing(grove)].sort(
    (one, other) => other.seasons.today - one.seasons.today,
  )[0];
  const most =
    busiest === undefined
      ? "No tree was written today; the grove is resting."
      : `${named(busiest)} grew most today, ${escaped(plural(busiest.seasons.today, "tree"))} written.`;
  return {
    title: `<a href="#every" data-place="every">${escaped(plural(all, "repository", "repositories"))}</a>, <em><a href="#growing" data-place="growing">${grew.toLocaleString("en-CA")} growing.</a></em>`,
    lede: [worth.length === 0 ? "Still air: nothing waits on you." : worth.join(" "), most].join(
      " ",
    ),
    tab:
      grove.worth.length === 0
        ? "Code"
        : `Code: ${plural(grove.worth.length, "thing")} worth a look`,
  };
}

const STATES: readonly Standing["state"][] = ["broken", "stale", "connecting", "live"];

/** The worst of the watches' standings, the one the page says. */
function worst(standings: Iterable<Standing>): Standing | undefined {
  return [...standings].sort(
    (one, other) => STATES.indexOf(one.state) - STATES.indexOf(other.state),
  )[0];
}

/** The foot's line: where it was read, how fresh, and what couldn't be. */
function footOf(
  grove: Grove | undefined,
  now: Standing | undefined,
  heard: Iterable<string>,
): string {
  const where = grove === undefined ? "" : `Read on ${grove.machine}. `;
  const problems = [...(grove?.unread ?? []), ...heard];
  const trouble = problems.length > 0 ? ` Couldn't read ${problems.join("; ")}.` : "";
  return `${where}${now === undefined ? "" : said(now)}${trouble}`;
}

export { footOf, headOf, worst };
