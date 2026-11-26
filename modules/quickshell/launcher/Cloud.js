.pragma library
.import "Sprouting.js" as Sprouting

// Start New Work in the cloud: In the Cloud, beside its In rows, starting
// the work on a machine that costs money, one linked with room or one the
// node's providers make for it (Joel, 2026-11-25: "we need full
// integation, from pi, from cli, from laucnher, from website. i should
// not have to think to use this"). The node decides which machine, from
// the need `cloud` (mycelium's sprout), so the launcher never names a
// network or a provider. It's offered only where a provider says it could
// make one (the agents daemon's offers.rs, the node's `Offers`).
//
// A whole path is a folder on this machine alone, so work for the cloud
// names where it starts by a place's name, which the far machine finds
// among its own places: a zone's //path, or a checkout's last step.

// What work for the cloud needs of where it starts, as the node takes it.
const NEEDS = ["cloud"];

// How long a start in the cloud is waited on: the node answers once the
// session has started, which waits on a machine being made and then up to
// five minutes for it to link, and the agents daemon waits eight minutes
// (sprout.rs's PATIENCE_ELSEWHERE), so this is half a minute past it.
const PATIENCE_MS = (8 * 60 + 30) * 1000;

// How long what the cloud offers is kept before it's asked again: a
// provider's program runs for each ask, up to half a minute, so it isn't
// asked at every opening.
const OFFERS_KEPT_MS = 10 * 60 * 1000;

// The most offers read, as the daemon passes no more.
const MOST_OFFERS = 20;

// One plain word, as the node reads a place's or a machine's name
// (mycelium's places.rs): letters, digits, -, _ and ., not opening with -.
const PLAIN = /^[A-Za-z0-9_.][A-Za-z0-9_.-]{0,199}$/u;

// Dots alone, this folder or the one above, which name no place or machine.
const DOTS = /^\.+$/u;

// A zone's name, as Wheres.js reads one.
const ZONE_NAMED = /^\/\/\S+$/u;

// Why a refusal that gives none is journalled, and why no answer is.
const NO_WHY = "the agents daemon gave no reason";
const NO_ANSWER = "the agents daemon isn't answering";

// What the agents daemon's `answer` to the offers ask changes, why the
// last went unoffered being `unoffered`: `{ offers, unoffered, journal }`,
// `offers` the shapes offered, each `{ shape, hourly, currency }` with its
// cost only where both are words, or null to keep the last; `journal` why
// to journal, said once as each new why comes, or null. A node that can't
// take the cloud's need (offers.rs's `untaken`) offers none, so what it
// offered before goes; any other error keeps the last.
function heard(unoffered, answer) {
    if (Array.isArray(answer?.offers)) {
        const offers = answer.offers.slice(0, MOST_OFFERS)
            .filter(one => typeof one?.shape === "string" && one.shape !== "")
            .map(one => typeof one.hourly === "string" && typeof one.currency === "string"
                ? { shape: one.shape, hourly: one.hourly, currency: one.currency }
                : { shape: one.shape });
        return { offers, unoffered: "", journal: null };
    }
    const why = answer === null || answer === undefined ? NO_ANSWER
        : String(answer?.error?.message ?? "").trim() || NO_WHY;
    const offers = answer?.error?.code === "untaken" ? [] : null;
    return { offers, unoffered: why, journal: why === unoffered ? null : why };
}

// Whether the cloud, as last `offered` (heard's offers), offers anything to
// start work on: a node offering nothing can't take a sprout's need.
function offering(offered) {
    return Array.isArray(offered) && offered.length > 0;
}

// The fields of an offer (heard) that In the Cloud's drawer reads.
const OFFERED = ["shape", "hourly", "currency"];

// Whether the offers heard `now` differ from those kept `before`: by how
// many, or by any one's shape, price or currency, so a provider pricing a
// shape anew is said at once rather than at the shell's next start.
function differs(before, now) {
    return before.length !== now.length
        || now.some((one, at) => OFFERED.some(field => before[at][field] !== one[field]));
}

// What the offers `kept`, `{ offers, at, unoffered }` as Agents.qml keeps
// them, become once `answer` comes `now` (heard): `{ offers, at, unoffered,
// changed, journal }`, `changed` where the offers differ, so Start New
// Work draws its rows again, and `at` moving only when offers came.
function answered(kept, answer, now) {
    const heardNow = heard(kept.unoffered, answer);
    const came = heardNow.offers !== null;
    return {
        offers: came ? heardNow.offers : kept.offers,
        at: came ? now : kept.at,
        unoffered: heardNow.unoffered,
        changed: came && differs(kept.offers, heardNow.offers),
        journal: heardNow.journal
    };
}

// Whether the offers `kept`, `{ asking, at, offers }`, are asked for `now`:
// never while an ask is out, else once they're stale, and, for a start
// Returned (`pressed`), whenever none are offered, so a Return refused
// for want of them always asks again.
function asksNow(kept, now, pressed) {
    return !kept.asking && (stale(kept.at, now) || pressed && !offering(kept.offers));
}

// What a start in the cloud is given as offered: the `offers` heard at
// `at` where some are offered, or none were, the node answering; else
// `unoffered`, the why the last ask came back with, even while another is
// out, so Joel is never told of a wait the last ask already ended; else,
// nothing heard yet, null (refused).
function offeredNow(at, offers, unoffered) {
    if (at > 0 && (offering(offers) || unoffered === "")) {
        return offers;
    }
    return unoffered === "" ? null : unoffered;
}

// Whether what the cloud offers, as last heard at `at`, is to be asked
// again at `now`: never heard, or kept past OFFERS_KEPT_MS.
function stale(at, now) {
    return !(at > 0) || now - at >= OFFERS_KEPT_MS;
}

// The cheapest of `offers` by the hour, `{ hourly, currency }`, among
// those in the first priced one's currency, so two currencies are never
// weighed against each other; null where none is priced.
function cheapest(offers) {
    const priced = (offers ?? []).filter(one => Number.isFinite(Number.parseFloat(one.hourly)));
    if (priced.length === 0) {
        return null;
    }
    const currency = priced[0].currency;
    return priced.filter(one => one.currency === currency).reduce((low, one) =>
        Number.parseFloat(one.hourly) < Number.parseFloat(low.hourly) ? one : low);
}

// The In the Cloud rows of Start New Work's `rows` (Wheres.wheres): each
// `{ folder, name, start }` again, its start now the place's name the far
// machine finds: a zone's //path as it is, or, for the folder of one of
// the node's `places` it calls a checkout, the folder's last step. A
// folder inside a checkout has no name there, so it isn't listed. Each
// start once, the first, so two checkouts of one name are one row.
function wheres(rows, places) {
    const checkouts = new Set((Array.isArray(places) ? places : [])
        .filter(one => one?.kind === "checkout").map(one => one.folder));
    const listed = [];
    const seen = new Set();
    for (const row of Array.isArray(rows) ? rows : []) {
        const last = String(row.folder).split("/").filter(step => step !== "").pop() ?? "";
        const start = ZONE_NAMED.test(row.start) ? row.start
            : checkouts.has(row.folder) && plain(last) ? last : "";
        if (start !== "" && !seen.has(start)) {
            seen.add(start);
            listed.push({ folder: row.folder, name: row.name, start });
        }
    }
    return listed;
}

// Whether `place` can be said to another machine: none, or a place's name.
function named(place) {
    const where = String(place ?? "");
    return where === "" || ZONE_NAMED.test(where) || plain(where);
}

// Whether `name` is one plain word that names something: PLAIN, not DOTS.
function plain(name) {
    return PLAIN.test(name) && !DOTS.test(name);
}

// The ask that sprouts `text` elsewhere, `on` saying where (onOf): the
// cloud, or a machine by its name, at the place named `place` where one is
// given; or null where nothing is said, the place isn't a name, `on` names
// nowhere, or it names the cloud while nothing is `offered` (offering),
// refused saying which of those last three.
function ask(text, place, on, offered) {
    const where = onOf(on);
    if (!named(place) || where === null || Object.keys(where).length === 0
        || (where.needs !== undefined && !offering(offered))) {
        return null;
    }
    const sprout = Sprouting.ask(text, "");
    const at = String(place ?? "");
    return sprout === null ? null : Object.assign(sprout, where, at !== "" ? { folder: at } : {});
}

// Why the ask for work elsewhere at `place`, `on` saying where with the
// cloud as last `offered`, was refused when its words weren't: by Start
// New Work's `words`, the place unnamed, the cloud not heard yet (null
// offered, as a link's act Returned before the first offers came), the
// why its last ask came back with no offers (a why offered, offeredNow),
// none offered by a node that takes the cloud, else where it starts
// misnamed.
function refused(place, on, offered, words) {
    if (!named(place)) {
        return words?.unnamed ?? "";
    }
    if (onOf(on)?.needs === undefined) {
        return words?.unmachined ?? "";
    }
    if (offered === null) {
        return words?.unheard ?? "";
    }
    if (typeof offered === "string") {
        // A function, so a `$` in the node's words is said as written.
        return String(words?.unoffered ?? "").replace("{why}", () => offered);
    }
    return offering(offered) ? words?.unmachined ?? "" : words?.unprovided ?? "";
}

// Where a link's `on` says its work starts, as Launcher.qml's newWorkOn
// is given it, "on:" before it: what follows, or "-", which names nowhere,
// where nothing does.
function linked(on) {
    const said = String(on ?? "");
    const machine = said.startsWith("on:") ? said.slice("on:".length) : "";
    return machine.trim() === "" ? "-" : machine;
}

// What `on`, a link's (linked) or an act's, asks of where its work
// starts: `{ needs }` for the cloud, `{ on }` for a machine by its name,
// `{}` for none, or null where it names nothing a start could go to.
function onOf(on) {
    const said = String(on ?? "").trim();
    if (said === "") {
        return {};
    }
    if (said === "cloud") {
        return { needs: NEEDS.slice() };
    }
    return plain(said) ? { on: said } : null;
}
