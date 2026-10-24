// How the network is doing, answered in the field: "network speed",
// "bandwidth", "traffic" or "ping". The headline is what the machine's own
// cards carry in and out now, a spark of it beside; a step in are In and
// Out, each with its spark, the Line (what the line carries at most, as the
// shaper last measured it, and how full it is), the Round trip (how long
// the internet takes to answer while the line is quiet, each server a step
// in), what's Dropped, and each Connection, the tunnels and bridges too.
//
// The counters are read from one kernel file, /proc/net/dev, when asked,
// and a rate is the difference between two reads, so the last is kept with
// its time. While the answer is shown it's read again once a second, and
// never more: nothing is read for a key, and nothing is spawned. What the
// line can carry is never measured here; it's read from the file the shaper
// writes, and a machine without the shaper simply has no Line or Round trip.
import qs
import Quickshell.Io
import QtQuick
import "Machine.js" as Machine
import "Numbers.js" as Numbers
import "Traffic.js" as Traffic

Provider {
    id: provider

    // The words it answers whole, and by their start (Provider.qml), and
    // a part each asks for alone.
    readonly property var parts: ({
        "ping": "Round trip",
        "latency": "Round trip",
        "round trip": "Round trip",
        "download speed": "In",
        "upload speed": "Out"
    })
    questions: ["network speed", "network performance", "internet speed", "bandwidth",
        "throughput", "traffic"].concat(Object.keys(parts))

    function answer(typed) {
        const question = asks(typed);
        if (question === "")
            return null;
        const part = parts[question];
        return part ? alone(part) : told();
    }

    // One part as an answer: its figure as the headline and its own parts a
    // step in.
    function alone(name) {
        const whole = told();
        const part = whole ? whole.rows.find(row => row.name === name) : null;
        if (!part)
            return null;
        return {
            headline: part.side,
            copy: "",
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
    // is every interface's counters by name. Read again once a second at
    // most, or a moment after a read with nothing recent before it, so a
    // rate stands for now as soon as it can.
    property var reads: null
    // Each spark's reads, the newest last, and the read they last took.
    property var series: ({ moved: [], in: [], out: [] })
    property var sparked: null
    // Whether each interface is a card of its own, asked once a name.
    property var physical: ({})
    readonly property int second: 1000
    readonly property int soonest: 200
    // Reads further apart than this aren't a rate of now, and start a
    // spark afresh.
    readonly property int stale: 5000

    function settled(kept) {
        return kept !== null && kept.before !== null && kept.last.at - kept.before.at <= stale;
    }

    function sample() {
        const now = Date.now();
        const kept = reads;
        const age = kept ? now - kept.last.at : Infinity;
        if (age >= second || (!settled(kept) && age >= soonest)) {
            reads = {
                last: { at: now, all: Traffic.counters(read(netdev)) },
                before: kept ? kept.last : null
            };
            if (!settled(reads))
                soon.restart();
        }
        return reads;
    }

    function isPhysical(name) {
        if (physical[name] === undefined) {
            probe.path = `/sys/class/net/${name}/device/uevent`;
            physical[name] = read(probe).trim() !== "";
        }
        return physical[name];
    }

    // What each interface is, beside its counters, leaving out the
    // shaper's mirrors.
    function described(all, mirrors) {
        const out = {};
        for (const name of Object.keys(all))
            out[name] = {
                kind: mirrors.includes(name) ? "" : Traffic.kind(name, isPhysical(name)),
                counters: all[name]
            };
        return out;
    }

    function moved(kept, mirrors) {
        const seconds = kept.before ? (kept.last.at - kept.before.at) / 1000
            : Number(read(up).trim().split(/\s+/)[0]);
        return Traffic.rates(described(kept.last.all, mirrors),
            kept.before ? described(kept.before.all, mirrors) : null, seconds);
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
            out: added(series.out, now.out)
        };
    }

    function since(kept) {
        return !kept.before ? "since the machine started"
            : settled(kept) ? "just now" : "since you last asked";
    }

    readonly property var named: ({ wifi: "Wi-Fi", cable: "Cable" })

    // How the shaper's search for the line's most is going, in Joel's
    // words.
    readonly property var finding: ({
        climbing: "as measured, still finding its most",
        holding: "as measured, held while the line's busy",
        cut: "as measured, cut while the line queued"
    })

    function told() {
        const kept = sample();
        if (Object.keys(kept.last.all).length === 0)
            return null;
        const status = parsed(read(shaper));
        const now = moved(kept, Traffic.mirrors(status));
        push(kept, now);
        const when = since(kept);
        const cards = now.each.filter(one => one.kind === "wifi" || one.kind === "cable");
        const shaped = cards.map(card => Traffic.line(status, card.name))
            .find(link => link !== null && link.follows === null) ?? null;
        const rows = [figure({
            name: "In",
            side: bits(now.in),
            graph: spark(series.in),
            note: when
        }), figure({
            name: "Out",
            side: bits(now.out),
            graph: spark(series.out),
            note: when
        })];
        if (shaped !== null) {
            const full = Math.max(shaped.in > 0 ? now.in / shaped.in : 0,
                shaped.out > 0 ? now.out / shaped.out : 0);
            rows.push(figure({
                name: "Line",
                side: `${bits(shaped.in)} in \u00b7 ${bits(shaped.out)} out`,
                graph: bar(full),
                note: finding[shaped.state] ?? "as measured"
            }));
            const typical = Traffic.typical(shaped.trips);
            if (typical !== null)
                rows.push(holder({
                    name: "Round trip",
                    side: ms(typical),
                    note: "while the line is quiet, the middle of "
                        + `${shaped.trips.length} servers`
                }, () => shaped.trips.map(trip => figure({
                    name: trip.server,
                    side: ms(trip.ms),
                    note: "while the line is quiet"
                }))));
        }
        rows.push(figure({
            name: "Dropped",
            side: now.dropped === 0 ? "none"
                : `${number(now.dropped, now.dropped < 10 ? 1 : 0)} packets/s`,
            note: when
        }));
        const listed = now.each.filter(one => named[one.kind] !== undefined
            || one.in + one.out > 0);
        if (listed.length > 1)
            rows.push(holder({
                name: "Connections",
                side: String(listed.length),
                note: "each card, tunnel and bridge, busiest first"
            }, () => listed.slice().sort((a, b) => b.in + b.out - (a.in + a.out))
                .map(one => figure({
                    name: named[one.kind] ?? one.name,
                    side: `${bits(one.in)} in \u00b7 ${bits(one.out)} out`,
                    note: named[one.kind] !== undefined ? one.name : when
                }))));
        return {
            headline: `${bits(now.in)} in \u00b7 ${bits(now.out)} out`,
            copy: "",
            graph: spark(series.moved),
            glyph: Theme.glyph.traffic,
            note: when,
            rows: rows,
            live: provider
        };
    }

    // While one of its answers is shown, it's asked again each second, so
    // its figures move; and once a moment after a first read, so a rate
    // stands for now as soon as it can.
    readonly property bool shown: launcher !== null && launcher.open && launcher.held.length === 0
        && launcher.answer?.live === provider

    Timer {
        interval: provider.second
        repeat: true
        running: provider.shown
        onTriggered: provider.answered()
    }

    Timer {
        id: soon
        interval: provider.soonest
        onTriggered: {
            if (provider.shown)
                provider.answered();
        }
    }

    FileView {
        id: netdev
        path: "/proc/net/dev"
        blockLoading: true
        printErrors: false
    }
    FileView {
        id: shaper
        path: "/run/shaper/status.json"
        blockLoading: true
        printErrors: false
    }
    FileView {
        id: up
        path: "/proc/uptime"
        blockLoading: true
        printErrors: false
    }
    FileView {
        id: probe
        blockLoading: true
        printErrors: false
    }
}
