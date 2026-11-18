// How the network is doing, answered in the field: "speed", Network's one
// word for it, or "round trip" alone. The headline is what the machine's own
// cards carry in and out now, a spark of it beside; a step in are In and
// Out, each with its spark, the Connection (what it carries at most, as the
// shaper last measured it, and how full it is), the Round trip (how long
// the internet takes to answer while the line is quiet, each server a step
// in), what's Dropped, and each Connection, the tunnels and bridges too.
//
// The counters are read from one kernel file, /proc/net/dev, and a rate is
// the difference between two reads, so the last is kept with its time. It's
// loaded in the background as the launcher opens and once a second while
// it's open, and an answer says what was last loaded, or, when that's
// old, that it's from when last read while it loads again: nothing is
// read or waited on for a key, and nothing is spawned. What the line can carry is
// never measured here; it's read from the file the shaper writes, loaded
// beside the counters, and a machine without the shaper simply has no
// Connection or Round trip.
import qs
import Quickshell.Io
import QtQuick
import "Interfaces.js" as Interfaces
import "Machine.js" as Machine
import "Numbers.js" as Numbers
import "Order.js" as Order
import "Traffic.js" as Traffic

Provider {
    id: provider

    // The words it answers whole, and by their start (Provider.qml), and
    // a part each asks for alone, by its id in the vocabulary (traffic:trip).
    readonly property var parts: ({
        "round trip": "trip"
    })
    questions: ["speed"].concat(Object.keys(parts))

    function answer(typed) {
        const question = asks(typed);
        if (question === "")
            return null;
        const part = parts[question];
        return part ? alone(part) : told();
    }

    // One part as an answer: its figure as the headline and its own parts a
    // step in.
    function alone(id) {
        const whole = told();
        const part = whole ? whole.rows.find(row => row.name === named(id)) : null;
        if (!part)
            return null;
        return {
            headline: part.side,
            graph: part.graph,
            glyph: whole.glyph,
            rows: part.verbs ? part.verbs() : [],
            note: part.note !== "" ? part.note : whole.note,
            live: provider
        };
    }

    function number(value, places) {
        return Numbers.format(value, places, settings.group, settings.point);
    }

    function bits(bytesPerSecond) {
        return Traffic.bits(bytesPerSecond, number);
    }

    function ms(value) {
        return `${number(value, value < 10 ? 1 : 0)} ms`;
    }

    readonly property int graphed: 20
    // A spark is drawn against at least this many bytes a second, a
    // megabit, so a quiet line draws low rather than its own noise full.
    readonly property real floor: 125000

    function spark(values) {
        return { shape: "spark", values: Machine.spark(values.slice(-graphed), floor), warn: false };
    }

    function bar(value, warn) {
        return { shape: "bar", values: [Math.max(0, Math.min(1, value))], warn: warn === true };
    }

    function figure(fields) {
        return copyRow(Object.assign({ copy: "", hue: "machine", glyph: "", note: "" }, fields));
    }

    // A part holding more wears the network speed's glyph, dimmed.
    function holder(fields, inside) {
        return holderRow(Object.assign({ glyph: Theme.glyph.traffic }, fields), inside);
    }

    // The last two reads, { last, before }, each { at, all }, where `all`
    // is every interface's counters by name, or null until the first load
    // lands. Loaded again a moment after a load with nothing recent before
    // it, so a rate stands for now as soon as it can.
    property var reads: null
    // Each spark's reads, the newest last, when each was read, and the
    // read they last took.
    property var series: ({ moved: [], in: [], out: [], at: [] })
    property var sparked: null
    // What each interface is, asked of sysfs once a name while it stays,
    // { kind, uevent, sure }, kept with the uevent it was read from:
    // forgotten once it's gone from the counters. After a gap between
    // reads it's no longer sure, since an interface can go and come again,
    // as a VPN's tunnel does, as something else; its uevent alone is read
    // again then, which names its index. The kernel gives each interface
    // made a new index unless whoever makes it asks for one, which none of
    // the tunnels here do, so the same uevent is the same interface; one
    // that can't be read proves nothing and has all four read again. Each
    // file is loaded in the background, one after another, never while an
    // answer waits: a step into Network asks after a gap each time, and a
    // blocking read of each took a median 6 to 14 ms and up to 133 under
    // load (2026-11-17), of a step that has 50 ms for all of it. Until its
    // files land, an interface first seen is left out, and one no longer
    // sure is what it was.
    property var interfaceKinds: ({})
    readonly property int second: 1000
    readonly property int soonest: 200
    // Reads further apart than this aren't a rate of now, and start a
    // spark afresh.
    readonly property int stale: 5000

    function settled(kept) {
        return kept !== null && kept.before !== null && kept.last.at - kept.before.at <= stale;
    }

    // Whether the last load is recent enough to say what's happening now.
    // An answer asked when it isn't, as a terminal's `launcher show` asks
    // with the launcher closed, or as it opens before its load lands, says
    // its figures are from when they were last read, loads them again, and
    // is still reading (Provider.stillReading), so a terminal asks again,
    // until a rate of now has landed and every interface's files with it.
    function current() {
        return reads !== null && Date.now() - reads.last.at <= stale;
    }

    function caughtUp() {
        if (stillReading && current() && settled(reads) && probing === null
            && asking.length === 0)
            stillReading = false;
    }

    // Where each file is, which its case points at files of its own
    // (tests/traffic-reads.qml), so it can tell an answer reads none.
    property string counterFile: "/proc/net/dev"
    property string statusFile: "/run/shaper/status.json"
    property string uptimeFile: "/proc/uptime"

    // When the load in flight was asked, or 0 with none.
    property real asked: 0

    // Loads the counters and the shaper's status again, in the background.
    // A FileView asked again while it loads loads again, and both land a
    // moment apart, a rate over that moment; so while one asked within the
    // last second is in flight, as an opening and an answer both ask, or
    // for a tenth of a second after one lands, nothing more is loaded.
    function look() {
        const now = Date.now();
        if (now - asked < second
            || reads !== null && Math.abs(now - reads.last.at) < soonest / 2)
            return;
        asked = now;
        netdev.reload();
        shaper.reload();
    }

    // A load of the counters landed: it's the last read now, and the one
    // it follows is before it. Each interface not sure what it is has its
    // files asked, and an answer shown is told.
    function counted() {
        const now = Date.now();
        const kept = reads;
        const age = kept ? now - kept.last.at : Infinity;
        asked = 0;
        const all = Traffic.counters(netdev.text());
        const known = {};
        for (const name of Object.keys(all)) {
            const kind = interfaceKinds[name];
            if (kind !== undefined)
                known[name] = age <= stale ? kind : Object.assign({}, kind, { sure: false });
        }
        interfaceKinds = known;
        reads = {
            last: { at: now, all: all },
            // A clock stepped back makes no rate; the read starts afresh.
            before: kept && age > 0 ? kept.last : null
        };
        const mirrors = Traffic.mirrors(parsed(shaper.text()));
        probeFor(Object.keys(all).filter(name => !mirrors.includes(name) && !known[name]?.sure));
        if (!settled(reads))
            soon.restart();
        caughtUp();
        if (shown)
            answered();
    }

    // The counters couldn't be loaded: nothing more is coming to wait for.
    function uncounted() {
        asked = 0;
        stillReading = false;
    }

    // Where sysfs keeps each interface's folder; its case gives a folder
    // of its own, since a build has no sysfs (tests/traffic-reads.qml).
    property string interfaces: "/sys/class/net"
    // How many sysfs files it has loaded of each interface, so its case
    // holds it to loading few.
    property var probed: ({})

    // The interfaces whose files are still to be asked, in turn, and the
    // one being asked, { name, files, said, done }: its uevent first, and
    // the other three only when that isn't the uevent its kind was read
    // from. One whose files have all landed is done, so it's asked again
    // if a load wants it.
    property var asking: []
    property var probing: null

    function probeFor(names) {
        const queued = asking.map(one => one.name)
            .concat(probing && !probing.done ? [probing.name] : []);
        asking = asking.concat(names.filter(name => !queued.includes(name))
            .map(name => ({ name: name, files: ["uevent"], said: {} })));
        if (probing === null)
            probeNext();
    }

    function probeNext() {
        probing = asking[0] ?? null;
        asking = asking.slice(1);
        if (probing !== null)
            probeFile();
        else
            caughtUp();
    }

    function probeFile() {
        probeWatch.restart();
        const name = probing.name;
        probed[name] = (probed[name] ?? 0) + 1;
        const path = `${interfaces}/${name}/${probing.files[0]}`;
        // Set again to the path it holds, the probe loads nothing, though
        // the file has changed, as an interface made again with no other
        // interface probed between has; let go of it first.
        if (probe.path === path)
            probe.path = "";
        probe.path = path;
    }

    // A file of the interface being asked landed, saying `text`, or ""
    // when it can't be read: the next file is asked, or the next
    // interface, and an answer shown is told once a kind has changed.
    function probeSaid(text) {
        const one = probing;
        if (one === null)
            return;
        one.said[one.files[0]] = text.trim();
        one.files = one.files.slice(1);
        const kept = interfaceKinds[one.name];
        const uevent = one.said.uevent;
        if (one.said.type === undefined && one.files.length === 0
            && (uevent === "" || kept?.uevent !== uevent))
            one.files = ["type", "tun_flags", "device/uevent"];
        if (one.files.length > 0) {
            Qt.callLater(probeFile);
            return;
        }
        const kind = one.said.type === undefined ? kept.kind : Interfaces.sysKind(one.name, {
            uevent: uevent,
            type: one.said.type,
            tun: one.said.tun_flags !== "",
            card: one.said["device/uevent"] !== ""
        });
        // One gone from the counters while it was asked is forgotten.
        if (reads?.last.all[one.name] !== undefined)
            interfaceKinds[one.name] = { kind: kind, uevent: uevent, sure: true };
        one.done = true;
        probeWatch.stop();
        Qt.callLater(probeNext);
        if (kind !== kept?.kind && shown)
            answered();
    }

    // What an interface is, as the addresses name it (Interfaces.js), so a
    // cable is Wired here as there, and a tunnel a VPN: as its files last
    // said, or "", left out, until they've been read.
    function interfaceKind(name) {
        return interfaceKinds[name]?.kind ?? "";
    }

    // What each interface is, beside its counters, leaving out the
    // shaper's mirrors; the read before needs only its counters.
    function described(all, mirrors) {
        const out = {};
        for (const name of Object.keys(all))
            out[name] = {
                kind: mirrors.includes(name) ? "" : interfaceKind(name),
                counters: all[name]
            };
        return out;
    }

    function countersOf(all) {
        const out = {};
        for (const name of Object.keys(all))
            out[name] = { counters: all[name] };
        return out;
    }

    function moved(kept, mirrors) {
        const seconds = kept.before ? (kept.last.at - kept.before.at) / 1000
            : Number(up.text().trim().split(/\s+/)[0]);
        return Traffic.rates(described(kept.last.all, mirrors),
            kept.before ? countersOf(kept.before.all) : null, seconds);
    }

    // Adds a read to the sparks once, starting them afresh after a gap. A
    // first read says only what's been since the machine started, so it
    // adds nothing.
    function push(kept, now) {
        if (kept.last === sparked || !kept.before)
            return;
        sparked = kept.last;
        const fresh = settled(kept);
        const added = (drawn, value) => (fresh ? drawn : []).concat([value]).slice(-graphed);
        series = {
            moved: added(series.moved, now.in + now.out),
            in: added(series.in, now.in),
            out: added(series.out, now.out),
            at: added(series.at, kept.last.at)
        };
    }

    // A word its drawers say, as the vocabulary has it, what's in braces
    // filled in: "{low} to {high}".
    function said(key, fill) {
        let text = Words.of("traffic").said?.[key] ?? "";
        for (const name of Object.keys(fill ?? {}))
            text = text.replace(`{${name}}`, String(fill[name]));
        return text;
    }

    // A sparked rate's low and high over the reads sparked, or steady once
    // they read the same, and how many reads over how long, as WARP's
    // tunnel says its own (WarpSeen.js). One read sparks nothing, so it
    // says none.
    function ranged(values, times) {
        if (values.length < 2)
            return [];
        const low = bits(Math.min(...values));
        const high = bits(Math.max(...values));
        const span = Math.round((times[times.length - 1] - times[0]) / second);
        return [{
            label: named("range"),
            value: low === high ? said("steady", { figure: low })
                : said("range", { low: low, high: high }),
            figure: spark(values)
        }, {
            label: named("reads"),
            value: said("reads", { count: number(values.length, 0),
                span: said("seconds", { figure: number(span, 0) }) })
        }];
    }

    // What every card has carried since the machine started, each way,
    // and dropped, from the counters' own totals, which no rate says.
    function carried(kept, cards) {
        const sum = key => cards.reduce((all, card) =>
            all + (kept.last.all[card.name]?.[key] ?? 0), 0);
        return { in: sum("in"), out: sum("out"), dropped: sum("dropped") };
    }

    function packets(count) {
        return said(count === 1 ? "packet" : "packets", { count: number(count, 0) });
    }

    // A drawer of the figures behind a row, its eyebrow saying once whose
    // they are: the machine's own cards, as the headline's are.
    function plate(title, facts) {
        return { eyebrow: said("eyebrow"), title: title, facts: facts };
    }

    function since(kept) {
        return !kept.before ? "since the machine started"
            : !current() ? "when last read"
            : settled(kept) ? "just now" : "since you last asked";
    }

    // A part's name, as the vocabulary says it: "Round trip" for traffic:trip.
    function named(id) {
        return Words.name(`traffic:${id}`);
    }

    // Each connection's name, as the addresses' answer says it, never the
    // kernel's, numbered when there are several of a kind: "VPN 2".
    function connectionNames(listed) {
        const kinds = listed.map(one => one.kind);
        return listed.map((one, at) => {
            const alike = kinds.filter(kind => kind === kinds[at]).length;
            const nth = kinds.slice(0, at + 1).filter(kind => kind === kinds[at]).length;
            const base = Words.name(`networks:${kinds[at]}`);
            return alike > 1 ? `${base} ${nth}` : base;
        });
    }

    // How the shaper's search for the connection's most is going, in
    // Joel's words.
    readonly property var finding: ({
        climbing: "as measured, still finding its most",
        holding: "as measured, held while the connection's busy",
        cut: "as measured, cut while the connection queued"
    })

    function told() {
        const kept = reads;
        if (!current()) {
            stillReading = true;
            look();
        }
        if (kept === null || Object.keys(kept.last.all).length === 0)
            return null;
        const status = parsed(shaper.text());
        const now = moved(kept, Traffic.mirrors(status));
        push(kept, now);
        const when = since(kept);
        const cards = now.each.filter(one => one.kind === "wifi" || one.kind === "wired");
        const shaped = cards.map(card => Traffic.line(status, card.name))
            .find(link => link !== null && link.follows === null) ?? null;
        const total = carried(kept, cards);
        // Each way's drawer says its range while shown and what it has
        // carried since the machine started, the line saying only now.
        const inward = series.in;
        const outward = series.out;
        const times = series.at;
        const rows = [figure({
            name: named("in"),
            side: bits(now.in),
            graph: spark(inward),
            note: when,
            drawer: () => plate(named("in"), ranged(inward, times).concat([
                { label: named("total"), value: Machine.bytes(total.in) }]))
        }), figure({
            name: named("out"),
            side: bits(now.out),
            graph: spark(outward),
            note: when,
            drawer: () => plate(named("out"), ranged(outward, times).concat([
                { label: named("total"), value: Machine.bytes(total.out) }]))
        })];
        if (shaped !== null) {
            const full = Math.max(shaped.in > 0 ? now.in / shaped.in : 0,
                shaped.out > 0 ? now.out / shaped.out : 0);
            const share = (rate, most) => most > 0
                ? `${number(100 * rate / most, 0)}%` : "\u2013";
            // A bar is a thing with more to say, the share each way of
            // what the line carries at most, where the bar draws only the
            // fuller.
            rows.push(figure({
                name: named("line"),
                side: `${bits(shaped.in)} in \u00b7 ${bits(shaped.out)} out`,
                graph: bar(full),
                note: finding[shaped.state] ?? "as measured",
                thing: true,
                drawer: () => plate(named("line"), [{
                    label: named("full"),
                    value: said("ways", { in: share(now.in, shaped.in),
                        out: share(now.out, shaped.out) })
                }, {
                    label: named("now"),
                    value: said("ways", { in: bits(now.in), out: bits(now.out) })
                }])
            }));
            const typical = Traffic.typical(shaped.trips);
            // The servers are kept quickest first (Traffic.line).
            const quickest = shaped.trips[0] ?? null;
            const slowest = shaped.trips[shaped.trips.length - 1] ?? null;
            if (typical !== null)
                rows.push(holder({
                    name: named("trip"),
                    side: ms(typical),
                    note: "while the connection is quiet, the middle of "
                        + `${shaped.trips.length} servers`,
                    thing: true,
                    drawer: () => plate(named("trip"), [{
                        label: named("quickest"),
                        value: `${quickest.server} \u00b7 ${ms(quickest.ms)}`
                    }, {
                        label: named("slowest"),
                        value: `${slowest.server} \u00b7 ${ms(slowest.ms)}`
                    }])
                }, () => shaped.trips.map(trip => figure({
                    name: trip.server,
                    side: ms(trip.ms),
                    note: "while the connection is quiet"
                }))));
        }
        const listed = now.each.filter(one => one.kind === "wifi" || one.kind === "wired"
            || one.in + one.out > 0);
        // Numbered in the kernel's order, so a name stays as the busiest
        // changes; the kernel's own name only finds it.
        const names = connectionNames(listed);
        const busiest = Order.amounts({ amount: each => each.one.in + each.one.out,
            name: each => each.name });
        const byBusiest = Order.sorted(listed.map((one, at) => ({ one: one, name: names[at] })),
            busiest);
        const rate = one => said("ways", { in: bits(one.in), out: bits(one.out) });
        const top = byBusiest[0] ?? null;
        // Dropped says, beside its rate now, how many each card has
        // dropped since the machine started, a card that dropped none
        // left out.
        const dropping = cards.map(card => ({
            name: names[listed.indexOf(card)] ?? card.name,
            count: kept.last.all[card.name]?.dropped ?? 0
        })).filter(card => card.count > 0);
        rows.push(figure({
            name: named("dropped"),
            side: now.dropped === 0 ? "none"
                : `${number(now.dropped, now.dropped < 10 ? 1 : 0)} packets/s`,
            note: when,
            thing: true,
            drawer: () => plate(named("dropped"), [{
                label: named("total"),
                value: packets(total.dropped)
            }].concat(dropping.length > 1 ? dropping.map(card => ({
                label: card.name,
                value: packets(card.count)
            })) : []))
        }));
        if (listed.length > 1)
            rows.push(holder({
                name: named("connections"),
                side: String(listed.length),
                note: "each card, VPN and bridge, busiest first",
                thing: true,
                drawer: () => plate(named("connections"), byBusiest.slice(0, 3)
                    .map(each => ({ label: each.name, value: rate(each.one) })))
            }, () => byBusiest.map(each => figure({
                name: each.name,
                keywords: each.one.name,
                side: rate(each.one),
                note: when
            }))));
        const headline = rate(now);
        // The headline's drawer says what the line carries over the reads
        // shown, the busiest connection, and what the cards have carried
        // since the machine started, the parts inside saying the rest.
        const behind = ranged(series.moved, times).concat(top !== null && listed.length > 1
            ? [{ label: named("busiest"), value: `${top.name} \u00b7 ${rate(top.one)}` }] : [])
            .concat([{
                label: named("total"),
                value: said("ways", { in: Machine.bytes(total.in), out: Machine.bytes(total.out) })
            }]);
        return {
            headline: headline,
            graph: spark(series.moved),
            glyph: Theme.glyph.traffic,
            note: when,
            rows: rows,
            thing: true,
            drawer: () => plate(headline, behind),
            live: provider
        };
    }

    // While one of its answers is shown, each load tells it, so its figures
    // move.
    readonly property bool shown: launcher !== null && launcher.open && launcher.held.length === 0
        && launcher.answer?.live === provider

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (provider.launcher.open)
                provider.look();
        }
    }

    // Loaded again each second while the launcher's open, so an answer
    // asked then says a rate of now, and a moment after a load with nothing
    // recent before it.
    Timer {
        interval: provider.second
        repeat: true
        running: provider.launcher !== null && provider.launcher.open
        onTriggered: provider.look()
    }

    Timer {
        id: soon
        interval: provider.soonest
        onTriggered: provider.look()
    }

    // A file that never lands, or a turn that throws, would leave the
    // probe waiting for good and every interface after it left out, so
    // one that takes this long is given up; the next load asks again.
    // A read that lands after it's given up is taken for the probe then
    // under way, if any, and corrected when that interface is next asked,
    // after the next gap.
    Timer {
        id: probeWatch
        interval: 2 * provider.second
        onTriggered: {
            provider.probing = null;
            probe.path = "";
            provider.probeNext();
        }
    }

    // None blocks, even on its first load, so nothing is ever waited on;
    // each loads as it's made, and uptime only then, for the first rate,
    // since the machine started.
    FileView {
        id: netdev
        path: provider.counterFile
        printErrors: false
        onLoaded: provider.counted()
        onLoadFailed: provider.uncounted()
    }
    FileView {
        id: shaper
        path: provider.statusFile
        printErrors: false
    }
    FileView {
        id: up
        path: provider.uptimeFile
        printErrors: false
    }
    FileView {
        id: probe
        printErrors: false
        onLoaded: provider.probeSaid(probe.text())
        onLoadFailed: provider.probeSaid("")
    }
}
