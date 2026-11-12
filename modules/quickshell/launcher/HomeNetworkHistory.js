// What the Home Network place's drawers draw of what the eero daemon
// keeps of the last hour and day (modules/eero/AGENTS.md, How It Reads):
// each figure's history as a spark beside it, a device's moves, how busy
// each radio is and how strongly it sends, this machine's own looks at
// the internet, the outages of 30 days, the devices that joined or left,
// and what happened, with what the daemon told. The record keeps a
// history as one list a figure, a sample a read, oldest first, null
// where eero gave none (modules/eero/src/model.rs).
// The pieces a fact is made of are here too, and HomeNetworkDrawers.js
// takes them from here, so the two files read a figure one way. Plain
// reading of the record, held in Qt's own engine (tests/home-network.qml).
.pragma library
.import "../When.js" as When
.import "HomeNetwork.js" as Home
.import "Interfaces.js" as Interfaces

// A spark has twenty places, as Graph draws one.
const PLACES = 20;
const BARS = 5;
const PERCENT = 100;
const SECOND = 1000;
const DOT = " \u00b7 ";
// A name's words kept on one line, so "laptop computer 1" never leaves its
// 1 alone on the next, as a figure keeps to its unit (FileFacts.js).
const KEPT = "\u00a0";
const SPACES = / /g;
// A kept name's last word a number, then a space: the drawer keeps a
// figure to the word after it, as a unit (Drawer.qml's unbroken), which
// would bind "laptop computer 1 joined" into one run too wide for its
// line, so a zero-width space after the number lets the line break there.
const NUMBER_ENDS = /(\u00a0\d+) /g;
// A figure eero's sentences run into its unit, "5.2GHz", kept to it with
// a no-break space, as every figure the drawer sets is.
const RUN_IN = /(\d)([GMk]Hz)\b/g;
// How a drawer names a day beside When.js's rule: yesterday and tomorrow
// in words, the rest as Qt's formatDate says them in the locale.
const DAYS = {
    yesterday: "yesterday",
    tomorrow: "tomorrow",
    date: (at, format) => Qt.formatDate(at, format),
};

// A moment as a drawer says it, its day as When.js says every day the
// launcher shows (VKCF3Z, 2026-11-08): today its time, "22:54"; else
// "yesterday 22:54", a weekday within six days, "Thursday 10:52", then
// "24 Sep 09:53" this year and "13 Nov 2020" in another.
function when(iso, now) {
    const at = new Date(iso ?? "");
    if (!iso || Number.isNaN(at.getTime())) {
        return "";
    }
    const day = When.day(at.getTime(), now, DAYS);
    const time = Qt.formatTime(at, "HH:mm");
    if (day === "") {
        return time;
    }
    return at.getFullYear() === new Date(now).getFullYear() ? `${day} ${time}` : day;
}

// A day alone, as a row's note says when a thing was: "today",
// "yesterday", "on Thursday" within six days, then "on 5 Nov" and "on 13
// Nov 2020", by When.js's rule as the drawers' moments are. HomeNetwork.js
// is laid out flat by checks where When.js isn't, so its internet row is
// given this rather than reaching When.js itself.
function dayOn(iso, now) {
    const at = new Date(iso ?? "");
    if (!iso || Number.isNaN(at.getTime())) {
        return "";
    }
    const day = When.day(at.getTime(), now, DAYS);
    if (day === "") {
        return "today";
    }
    return day === DAYS.yesterday ? day : `on ${day}`;
}

function known(value) {
    return value !== null && value !== undefined && value !== "";
}

// Parts said together, those not known left out.
function joined(parts) {
    return parts.filter((part) => known(part)).join(DOT);
}

// A fact, or nothing where its value isn't known.
function fact(id, value, extra) {
    return known(value) ? Object.assign({ id, value: String(value) }, extra ?? {}) : null;
}

function kept(facts) {
    return facts.filter((one) => one !== null);
}

// A switch as words: "on" or "off".
function onOff(value) {
    if (!known(value)) {
        return "";
    }
    return value ? "on" : "off";
}

// A share as Meter's stroke, warning past `warn` where it's given.
function stroke(share, warn) {
    const value = Math.max(0, Math.min(1, share));
    return { shape: "stroke", values: [value], warn: warn !== undefined && value >= warn };
}

// A history's places: each sample its own while twenty hold them all,
// else the samples gathered into twenty runs, each the mean of the
// figures eero gave in it, so a spark spans the whole hour the daemon
// keeps rather than its last ten minutes, read every 30 seconds.
function fitted(values) {
    const one = (value) => (Number.isFinite(value) ? value : null);
    if (values.length <= PLACES) {
        return values.map(one);
    }
    const places = [];
    for (let at = 0; at < PLACES; at += 1) {
        const run = values.slice(Math.floor((at * values.length) / PLACES),
            Math.floor(((at + 1) * values.length) / PLACES)).filter((value) => one(value) !== null);
        places.push(run.length > 0 ? run.reduce((sum, value) => sum + value, 0) / run.length
            : null);
    }
    return places;
}

// A history as Graph's spark, which the drawer draws in its text's ink
// on a muted track: a gap where eero gave nothing, so a read missed
// isn't drawn as nothing moving, and each place a share of `most`, or of
// the history's own most where no scale is given; nothing until two
// places hold a figure.
function historySpark(values, most) {
    const places = fitted(values ?? []);
    const read = places.filter((value) => value !== null);
    if (read.length < 2) {
        return null;
    }
    const top = most ?? Math.max(...read);
    return {
        shape: "spark",
        values: places.map((value) => (value === null ? null : top > 0 ? value / top : 0)),
        warn: false,
    };
}

// A history scaled by `share` first, a gap kept where eero gave nothing.
function shares(values, share) {
    return (values ?? []).map((value) => (known(value) ? share(value) : null));
}

// Two histories added, a place known where either is: a device's speed
// both ways together.
function together(one, other) {
    return one.map((value, at) => (known(value) || known(other[at])
        ? (value ?? 0) + (other[at] ?? 0)
        : null));
}

// A device's sparks from its history: its signal from the faintest heard
// to the strongest, its SNR, each way's rate, and its speed both ways
// together. Each is null where the history holds fewer than two figures.
function deviceSparks(one) {
    const history = one.history ?? {};
    return {
        signal: historySpark(shares(history.signal_dbm, (dbm) => Interfaces.level(dbm)), 1),
        snr: historySpark(history.snr_db),
        rx: historySpark(history.rx_mbps),
        tx: historySpark(history.tx_mbps),
        speed: historySpark(together(history.down_mbps ?? [], history.up_mbps ?? [])),
    };
}

// A device's signal as a spark: of its last hour, else of what the shell
// read since it started, where the record keeps no history; nothing
// where neither has two readings.
function signalSpark(one, readings) {
    const own = deviceSparks(one).signal;
    if (own !== null || readings.length < 2) {
        return own;
    }
    return historySpark(readings.slice(-PLACES).map((dbm) => Interfaces.level(dbm)), 1);
}

// A node's name by its id, or the id where the record lists no such node.
function nodeNamed(model, id) {
    const node = (model?.nodes ?? []).find((each) => each.id === id) ?? null;
    return node ? Home.nodeName(node) : id;
}

// One thing then another, "Study to Den", or the one it is now where it
// didn't change or what it was isn't known.
function fromTo(from, to) {
    if (!known(from) || from === to) {
        return to ?? "";
    }
    return known(to) ? `${from} to ${to}` : from;
}

// A device's moves over the last day, newest first, each by when, as the
// events beside them are: from one node to another, or from one band to
// another on one node, "Study to Den \u00b7 2.4 GHz to 5 GHz".
function roams(model, one, now) {
    const moves = one.roams ?? [];
    if (moves.length === 0) {
        return [];
    }
    return [{ heading: "roams" }].concat(moves.map((move) => ({
        label: when(move.at, now),
        value: joined([fromTo(nodeNamed(model, move.from_node), nodeNamed(model, move.node)),
            fromTo(move.from_band, move.band)]),
    })));
}

// This machine's last look at the internet, each target's time beside a
// spark of the last hour's looks, a gap where one went unanswered: the
// gateway node, timed only from home, the quicker of two public
// resolvers, and the gateway node looking a name up.
function probe(net, now, settings) {
    const last = net.probe ?? null;
    if (last === null) {
        return [];
    }
    const looks = net.history?.probes ?? {};
    const ms = (value) => (known(value) ? `${Home.significant(value, settings)} ms` : null);
    return kept([
        { heading: "probe" },
        fact("looked", joined([when(last.at, now), last.home ? "from home" : "away from home",
            last.reachable ? "" : "internet not reached"])),
        fact("to-gateway", ms(last.gateway_ms), { figure: historySpark(looks.gateway_ms) }),
        fact("to-internet", ms(last.internet_ms), { figure: historySpark(looks.internet_ms) }),
        fact("lookup", ms(last.dns_ms), { figure: historySpark(looks.dns_ms) }),
    ]);
}

// Whether the record keeps a history of the internet, which a daemon
// that looks at it does with every read.
function keepsHistory(net) {
    return (net.history?.probes?.at ?? []).length > 0;
}

// The internet's outages as a count of what it counts: those of the
// last 30 days the history keeps, else those eero recorded.
function outagesSaid(net) {
    if (keepsHistory(net)) {
        const count = (net.history.outages ?? []).length;
        return count === 0 ? "none in 30 days" : `${count} in 30 days`;
    }
    const recorded = (net.outages ?? []).length;
    return recorded === 0 ? "none recorded" : `${recorded} recorded`;
}

// How long an outage lasted, or has so far while it lasts, where its
// start is known.
function outageLasted(outage, now) {
    if (!outage.start) {
        return "";
    }
    const end = outage.end ? Date.parse(outage.end) : now;
    const said = Interfaces.lasted(Math.max(0, Math.round((end - Date.parse(outage.start))
        / SECOND)));
    return outage.end ? said : `${said} so far`;
}

// Each outage of the last 30 days, newest first as the record keeps
// them, by when it began, how long it lasted and who saw it: eero, or
// this machine's own looks. eero's record gives no start in a shape the
// record reads yet, so one of eero's says its time isn't given.
function outages(net, now) {
    const list = net.history?.outages ?? [];
    if (list.length === 0) {
        return [];
    }
    return [{ heading: "outage-list" }].concat(list.map((outage) => ({
        label: outage.start ? when(outage.start, now) : "time not given",
        value: joined([outageLasted(outage, now),
            outage.seen_by === "probe" ? "seen by this machine" : "seen by eero"]),
    })));
}

// A name with its words kept on one line.
function whole(name) {
    return String(name).replace(SPACES, KEPT);
}

// A sentence as the drawer sets eero's: its words and names eero's, each
// device's name in it kept on one line, the longest first so a name inside
// another is kept as part of it, and its figures ours, "5.2\u00a0GHz".
function sentence(model, text) {
    const names = (model?.devices ?? []).concat(model?.devices_off ?? [])
        .map((one) => Home.nameOf(one)).filter((name) => known(name) && name.includes(" "))
        .sort((a, b) => b.length - a.length);
    return breakable(names.reduce((said, name) => said.split(name).join(whole(name)),
        String(text ?? "").replace(RUN_IN, `$1${KEPT}$2`)));
}

// Words a kept name's last number may break after, where a space follows it.
function breakable(text) {
    return text.replace(NUMBER_ENDS, "$1\u200b ");
}

// What happened over the last 30 days as one list, newest first: eero's
// activity in its own words, and each change the daemon told as a notice,
// its summary then what the notice said, a sentence as eero's are, so the
// drawer reads as `eero events` does. Each is { at, text }, as an event.
function timeline(model) {
    const told = (model?.told ?? []).map((one) => ({
        at: one.at,
        text: [one.summary, one.body].filter(known).join(". "),
    }));
    return (model?.events ?? []).concat(told)
        .sort((a, b) => Date.parse(b.at) - Date.parse(a.at));
}
