// Which rows hold a kind, by name or by a pattern, and where each thing of it is home, by the
// rules the system daemon's homes keep (modules/system/src/homes.rs).

import assert from "node:assert/strict";
import { test as it } from "node:test";
import { holdingOf, homesFrom, patterned } from "../homes.ts";

const machines = { kind: "machine", address: "#machine={name}" };
const held = { pattern: "mycelium/#machine={name}" };

/** A reader of each site's homes from `records`, by its path, failing for any it lacks. */
function reading(records: Record<string, unknown>): (site: string) => Promise<unknown> {
  return (site) =>
    site in records ? Promise.resolve(records[site]) : Promise.reject(new Error(`no ${site}`));
}

/** A site's homes, each thing a cell. */
function cells(...things: Record<string, unknown>[]): unknown {
  return { things: things.map((each) => ({ kind: "cell", ...each })) };
}

it("holds a kind by every row that names it, in the rows' order, whatever each is named", () => {
  const sites = {
    sites: [
      { path: "code/", name: "Code", homes: ["repository", "tree"] },
      { path: "mycelium/", name: "Your mycelium, now", homes: ["cell"] },
      { path: "notes/", name: "mycelium" },
      { path: "system/", homes: ["cell"] },
    ],
  };
  assert.deepEqual(holdingOf(sites, "cell"), { sites: ["mycelium/", "system/"] });
  assert.deepEqual(holdingOf(sites, "tree"), { sites: ["code/"] });
});

it("finds no holder for a kind no row claims, or in a record that isn't one", () => {
  const unclaimed = { sites: [{ path: "mycelium/", name: "Your mycelium, now" }] };
  assert.equal(holdingOf(unclaimed, "cell"), undefined);
  assert.equal(holdingOf({ sites: [{ homes: ["cell"] }] }, "cell"), undefined);
  assert.equal(holdingOf({ sites: "none" }, "cell"), undefined);
  assert.equal(holdingOf(null, "cell"), undefined);
});

it("takes a row's path only as steps of plain bytes under the sites' root", () => {
  const refused = ["../up/", "/rooted/", "./here/", ".drafts/", "%2e%2e/", "https://x/", "a\\b/"];
  for (const path of [...refused, "a//b/", "", "/"]) {
    const sites = { sites: [{ path, homes: ["cell"] }] };
    assert.equal(holdingOf(sites, "cell"), undefined, `${path} was taken as a home`);
  }
  for (const path of ["mycelium", "my-site_1.~/", "maps/cells/"]) {
    const sites = { sites: [{ path, homes: ["cell"] }] };
    assert.deepEqual(holdingOf(sites, "cell"), { sites: [path] }, `${path} was refused`);
  }
});

it("takes a pattern a row says, written from its page or from the sites' root", () => {
  const fromPage = { sites: [{ path: "mycelium/", homes: [machines] }] };
  const fromRoot = {
    sites: [{ path: "mycelium/", homes: [{ ...machines, address: "mycelium/#machine={name}" }] }],
  };
  assert.deepEqual(holdingOf(fromPage, "machine"), held);
  assert.deepEqual(holdingOf(fromRoot, "machine"), held);
});

it("lets a pattern hold its kind against its own row's name, said before it or after", () => {
  for (const homes of [
    [machines, "machine"],
    ["machine", machines],
  ]) {
    const sites = {
      sites: [
        { path: "mycelium/", homes },
        { path: "system/", homes: ["machine"] },
      ],
    };
    assert.deepEqual(holdingOf(sites, "machine"), held, JSON.stringify(homes));
  }
});

it("lets no pattern take a kind an earlier row has named", () => {
  const sites = {
    sites: [
      { path: "system/", homes: ["machine"] },
      { path: "mycelium/", homes: [machines, "machine"] },
    ],
  };
  assert.deepEqual(holdingOf(sites, "machine"), { sites: ["system/", "mycelium/"] });
});

it("lets a pattern it can't take claim nothing, so the next row serving its kind does", () => {
  for (const address of ["#machine", "other/#machine={name}", "?machine={name}", '#m="{name}"']) {
    const sites = {
      sites: [
        { path: "mycelium/", homes: [{ kind: "machine", address }] },
        { path: "system/", homes: ["machine"] },
      ],
    };
    assert.deepEqual(holdingOf(sites, "machine"), { sites: ["system/"] }, `${address} was taken`);
  }
});

it("puts a name in a pattern with every byte but the unreserved ones encoded", () => {
  assert.equal(patterned("m/#machine={name}", "kestrel"), "m/#machine=kestrel");
  assert.equal(patterned("m/#machine={name}", "a b/c&d'é"), "m/#machine=a%20b%2Fc%26d%27%C3%A9");
  assert.equal(patterned("m/#machine={name}", ""), undefined);
});

it("homes a thing by a pattern with no record read", async () => {
  const sites = { sites: [{ path: "mycelium/", homes: [{ kind: "cell", address: "#c={name}" }] }] };
  const read: string[] = [];
  const { homes, whole } = await homesFrom(sites, "cell", (site) => {
    read.push(site);
    return Promise.resolve(undefined);
  });
  assert.deepEqual(read, []);
  assert.equal(whole, true);
  assert.equal(homes("E2ZIAQ"), "mycelium/#c=E2ZIAQ");
  assert.equal(homes(""), undefined);
});

it("homes a thing by each name it has, from each row's record, the first standing", async () => {
  const sites = {
    sites: [
      { path: "gone/", homes: ["cell"] },
      { path: "mycelium/", homes: ["cell"] },
      { path: "system/", homes: ["cell"] },
    ],
  };
  const records = {
    "mycelium/": cells({
      key: "E2ZIAQ",
      name: "heron",
      names: ["egret"],
      address: "mycelium/#c=1",
    }),
    "system/": cells(
      { key: "E2ZIAQ", address: "system/#c=2" },
      { key: "Q3NEWS", address: "system/#c=3" },
    ),
  };
  const { homes, whole } = await homesFrom(sites, "cell", reading(records));
  // gone/'s record couldn't be read, so the ask isn't whole, but the others still count.
  assert.equal(whole, false);
  for (const name of ["E2ZIAQ", "heron", "egret"]) {
    assert.equal(homes(name), "mycelium/#c=1", name);
  }
  assert.equal(homes("Q3NEWS"), "system/#c=3");
});

it("takes from a record only the things of the kind asked, each with a name", async () => {
  const sites = { sites: [{ path: "mycelium/", homes: ["cell", "network"] }] };
  const records = {
    "mycelium/": {
      things: [
        { kind: "network", key: "repo.core", address: "mycelium/#n=core" },
        { kind: "cell", key: "", name: "", address: "mycelium/#c=" },
        { kind: "cell", key: "NOWHERE" },
      ],
    },
  };
  const { homes } = await homesFrom(sites, "cell", reading(records));
  for (const name of ["repo.core", "", "NOWHERE"]) {
    assert.equal(homes(name), undefined, name);
  }
});

it("takes no address off its site's own page, or one whose bytes could break out", async () => {
  const sites = { sites: [{ path: "mycelium/", homes: ["cell"] }] };
  const addresses = ["elsewhere/#c=A", "mycelium/other.html#c=A", "mycelium#c=A"];
  const breaking = ["\\", '"', "<", ">", "`", " ", "é", "\n"].map((each) => `mycelium/#c=${each}`);
  for (const address of [...addresses, ...breaking]) {
    const { homes } = await homesFrom(
      sites,
      "cell",
      reading({ "mycelium/": cells({ key: "A", address }) }),
    );
    assert.equal(homes("A"), undefined, `${address} was taken`);
  }
});
