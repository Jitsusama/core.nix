// Where another site is home to a thing, as the sites' rows say it, by the rules the system
// daemon's homes keep (modules/system/src/homes.rs), so a cell Code names opens the same page
// every surface opens. A row claims a kind by name, its things' homes then read from its
// homes.json, or by a pattern, every thing of the kind home at an address its name goes into.

import { isObject, text } from "../../local-sites/kit/records.ts";

/** Where a thing is home, by its name, when a site says; undefined where none does. */
type Homes = (name: string) => string | undefined;

/** Homes where no site says any thing is home. */
const NOWHERE: Homes = () => undefined;

/** What one ask brought: where things are home, and whether every record it needed read. */
interface Asked {
  readonly homes: Homes;
  readonly whole: boolean;
}

/**
 * How the sites hold a kind: by a pattern, its address from the sites' root with `{name}` where
 * a thing's name goes, or by the records of every row that names it, in the rows' order.
 */
type Holding = { readonly pattern: string } | { readonly sites: readonly string[] };

// Where a pattern's address puts the name a thing is known by.
const NAME = "{name}";
// The bytes a name or a site's step keeps as they are; every other is refused or encoded.
const UNRESERVED = /^[A-Za-z0-9\-._~]*$/u;
const VISIBLE = /^[\x21-\x7e]*$/u;
const BREAKING = /[\\"<>`]/u;
const HERE = /^\.\//u;

/**
 * Whether an address's bytes are visible ASCII with no backslash, double quote, angle bracket or
 * backtick, so it can't break out of a page's markup or a terminal's link.
 */
function plain(address: string): boolean {
  return VISIBLE.test(address) && !BREAKING.test(address);
}

/**
 * Whether a site's path stays under the sites' root, so a row can only name a site's own record
 * and an address it starts stays there: steps of letters, digits and `-._~` only, none empty but
 * the last and none starting with a dot, so no scheme, host, backslash, encoded or dotted step
 * can ride in on it, and none the sites' server refuses to serve.
 */
function inside(path: string): boolean {
  const steps = path.endsWith("/") ? path.slice(0, -1) : path;
  return (
    steps !== "" &&
    steps.split("/").every((step) => step !== "" && !step.startsWith(".") && UNRESERVED.test(step))
  );
}

/**
 * A claim's pattern, from the sites' root, when it names the thing and is a fragment on its
 * row's own page, its bytes plain. It may be written from the row's page, `#cell={name}`, or
 * from the root, `mycelium/#cell={name}`; a fragment can't change the page a browser opens, so
 * neither it nor a name put in it steps off the site.
 */
function patternOf(claim: unknown, path: string): string | undefined {
  const written = isObject(claim) ? text(claim, "address") : undefined;
  const address = written?.startsWith(path) ? written.slice(path.length) : written;
  const holds = address?.startsWith("#") && plain(address) && address.includes(NAME);
  return holds ? `${path}${address}` : undefined;
}

/**
 * How `rows`, the sites' rows, hold `kind`. A pattern holds it when no earlier row has claimed
 * it, against every later row and its own row's name; otherwise every row that names it gives
 * its record. A row that leaves the sites' folders, or a pattern it can't take, claims nothing,
 * so the next row serving the kind does, as the record keeps them newest first.
 */
function holdingOf(rows: unknown, kind: string): Holding | undefined {
  const listed = isObject(rows) && Array.isArray(rows["sites"]) ? rows["sites"] : [];
  const sites: string[] = [];
  for (const row of listed) {
    // One leading ./ names the sites' root itself, as the system's homes take it.
    const path = isObject(row) ? text(row, "path")?.replace(HERE, "") : undefined;
    const claims = isObject(row) && Array.isArray(row["homes"]) ? row["homes"] : [];
    if (path === undefined || !inside(path)) {
      continue;
    }
    const pattern = claims
      .filter((claim: unknown) => isObject(claim) && text(claim, "kind") === kind)
      .map((claim: unknown) => patternOf(claim, path))
      .find((each) => each !== undefined);
    if (pattern !== undefined && sites.length === 0) {
      return { pattern };
    }
    if (claims.includes(kind)) {
      sites.push(path);
    }
  }
  return sites.length === 0 ? undefined : { sites };
}

/** `pattern` with `name` in place, every byte of it but the unreserved ones percent-encoded. */
function patterned(pattern: string, name: string): string | undefined {
  if (name === "") {
    // A pattern would give a thing with no name a link that names nothing.
    return undefined;
  }
  const encoded = [...new TextEncoder().encode(name)]
    .map((byte) => String.fromCodePoint(byte))
    .map((each) =>
      UNRESERVED.test(each)
        ? each
        : `%${each.codePointAt(0)?.toString(16).toUpperCase().padStart(2, "0")}`,
    )
    .join("");
  return pattern.replaceAll(NAME, encoded);
}

/**
 * Adds to `found` where each thing of `kind` is home, by every name it's known by (its key, its
 * name and its other names), as `site`'s homes list them, a name already found keeping its
 * address: only an address that is a fragment on the site's own page, its bytes plain.
 */
function homesOf(homes: unknown, kind: string, site: string, found: Map<string, string>): void {
  const things = isObject(homes) && Array.isArray(homes["things"]) ? homes["things"] : [];
  for (const thing of things) {
    const address = isObject(thing) ? text(thing, "address") : undefined;
    if (
      !isObject(thing) ||
      text(thing, "kind") !== kind ||
      address === undefined ||
      !address.startsWith(`${site}#`) ||
      !plain(address)
    ) {
      continue;
    }
    const others = Array.isArray(thing["names"]) ? thing["names"] : [];
    for (const name of [thing["key"], thing["name"], ...others]) {
      if (typeof name === "string" && name !== "" && !found.has(name)) {
        found.set(name, address);
      }
    }
  }
}

/**
 * Where each thing of `kind` is home, as `rows` hold it: by its pattern, or by the records
 * `read` brings of every row naming it, the first row's address standing. A record `read`
 * rejects gives nothing, the other rows' still count, and the ask isn't whole.
 */
async function homesFrom(
  rows: unknown,
  kind: string,
  read: (site: string) => Promise<unknown>,
): Promise<Asked> {
  const holding = holdingOf(rows, kind);
  if (holding === undefined) {
    return { homes: NOWHERE, whole: true };
  }
  if ("pattern" in holding) {
    return { homes: (name) => patterned(holding.pattern, name), whole: true };
  }
  const said = await Promise.allSettled(holding.sites.map((site) => read(site)));
  const found = new Map<string, string>();
  for (const [at, site] of holding.sites.entries()) {
    const each = said[at];
    homesOf(each?.status === "fulfilled" ? each.value : undefined, kind, site, found);
  }
  const whole = said.every((each) => each.status === "fulfilled");
  return { homes: (name) => found.get(name), whole };
}

export { type Asked, type Holding, type Homes, holdingOf, homesFrom, NOWHERE, patterned };
