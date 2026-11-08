// How hot the machine runs, answered in the field by Heat's words
// ("temperature", "heat", "hot", "thermal", "sensors" or "fans", as
// words.json lists them). The headline is the hottest part and the fans;
// the rows are each part by its name first, then what it reads: the
// Processor, how often it's Throttled, its NPU, Image Processor, Media
// Engine and Display Engine, the Disk, the Memory, the Wi-Fi card, the
// Battery, the Air inside and each fan. Each temperature is drawn
// against the most its chip says it may reach, warning near it, beside
// what's left to its limit, and holds, a step in, its Last Reads
// while shown, its other sensors (a disk's second, hottest one), the
// processor's Cores, its High, Critical, Emergency and Low points and its
// chip's Alarm; its drawer holds where the firmware acts on its heat, the
// processor's cooling steps, its chip's electricity and the chip and
// zones it's read by. A fan is drawn against what it's rated for and
// holds its history, its Power, what it's Aiming for, its Least Speed and
// what has Control of it. Throttled counts the
// times the processor slowed itself to cool down, as the whole processor
// and core by core.
//
// The readings are the machine daemon's (modules/machine), its sensor,
// zone, cooling and processor records, which it reads once a second once
// Heat is asked, until the launcher closes, and holds here, so nothing is read on the shell's
// thread; Heat.js makes them into the parts. The
// processor's NPU, image processor and media and display engines are
// measured by no hwmon chip, so they come from its own telemetry
// (ProcessorTelemetry.qml), asked only while an answer is shown and listed
// from its first reading, a second after.
// Machine's processor and disk each say their temperature as one figure,
// the same reading drawn the same way (Heat.js's level); every finer
// detail, the cores, the disk's other sensors, the limits, the history and
// the throttling, is said only here. A sensor whose chip doesn't say what
// it measures is left out, so a reading is never shown twice under two
// names, and a thermal zone is read only for a part no chip gives.
import qs
import Quickshell
import QtQuick
import "Heat.js" as Heat
import "Machine.js" as Machine
import "Numbers.js" as Numbers

Provider {
    id: provider

    examples: ["temperature", "fans"]

    // The words it answers whole, and by their start (Provider.qml).
    questions: Words.answers("heat")

    // A part asked for alone, whole, by its own words ("throttling"), as
    // the vocabulary's id for it. Only whole, so the start of one never
    // takes another answer's start.
    readonly property var partsAsked: {
        const out = {};
        for (const id of ["heat:throttled"])
            for (const question of Words.answers(id))
                out[question] = id;
        return out;
    }

    function answer(typed) {
        const part = partsAsked[plainly(typed)];
        if (!part && asks(typed) === "")
            return null;
        if (!askedSinceOpen)
            Qt.callLater(() => provider.askedSinceOpen = true);
        return part ? alone(part) : told();
    }

    // Whether Heat has been asked since the launcher opened, as Machine's
    // place asks it for its Heat row, so the daemon reads the sensors only
    // then rather than whenever the launcher is open.
    property bool askedSinceOpen: false

    function number(value, places) {
        return Numbers.format(value, places ?? 0, settings.group, settings.point);
    }

    function degrees(value) {
        return `${number(value)} \u00b0C`;
    }

    function rpm(value) {
        return `${number(value)} rpm`;
    }

    function times(count) {
        return count === 0 ? "never" : count === 1 ? "once" : `${number(count)} times`;
    }

    // A length of time in milliseconds, as a clock reads it at a glance.
    function lasted(ms) {
        return ms < 1000 ? `${number(ms)} ms`
            : ms < 120000 ? `${number(ms / 1000, ms < 10000 ? 1 : 0)} s`
            : ms < 7200000 ? `${number(ms / 60000)} min`
            : `${number(ms / 3600000, 1)} h`;
    }

    // How rows are drawn, as Machine draws its own: a figure says
    // something and copies nothing; a part holding more (holderRow) steps
    // in on Return as on the right arrow, wearing its glyph dimmed.
    function figure(fields) {
        return copyRow(Object.assign({ copy: "", hue: "machine", glyph: "", note: "" }, fields));
    }

    // Graphs, each ten characters wide: a bar has one value, a strip one a
    // member, and a spark one a read, the newest last.
    readonly property int graphed: 20

    function bar(value, warn) {
        return { shape: "bar", values: [Math.max(0, Math.min(1, value))], warn: warn === true };
    }

    function strip(values, warn) {
        const per = Math.max(1, Math.ceil(values.length / graphed));
        const drawn = [];
        for (let at = 0; at < values.length; at += per)
            drawn.push(Math.max(...values.slice(at, at + per)));
        return { shape: "strip", values: drawn, warn: warn === true };
    }

    function spark(values) {
        return { shape: "spark", values: values.slice(-graphed), warn: false };
    }

    // A temperature's bar, against its most, warning near it.
    function level(value, part) {
        const said = Heat.level(value, part.high, part.crit, part.id);
        return bar(said.value, said.warn);
    }

    // The last two reads, { last, before }, each { at, said }, read again
    // once a second at most, or a moment after a read with nothing recent
    // before it, so a spark has a second read as soon as it can.
    property var reads: null
    // Each spark's reads, the newest last, by its part.
    property var series: ({})
    readonly property int second: 1000
    readonly property int soonest: 200
    // Reads further apart than this start a spark afresh.
    readonly property int stale: 5000

    function settled(kept) {
        return kept !== null && kept.before !== null && kept.last.at - kept.before.at <= stale;
    }

    function due(kept, now) {
        if (!kept)
            return true;
        const age = now - kept.last.at;
        return age >= second || (!settled(kept) && age >= soonest);
    }

    // The daemon's records of a kind, as held now.
    function heldOf(kind) {
        const held = daemon.revision >= 0 ? daemon.held : {};
        return Object.keys(held).filter(key => key.startsWith(`${kind}:`))
            .map(key => held[key]);
    }

    // Everything Heat says, as the daemon holds it: each chip's
    // temperatures (of a chip measuring many, only those it can name) and
    // fans, a thermal zone for a part no chip gives, and the throttling
    // counts.
    function sensed() {
        const chips = Heat.chipsOf(heldOf("sensor"));
        const zones = Heat.zonesOf(heldOf("zone"));
        const processor = heldOf("processor")[0] ?? null;
        return {
            parts: Heat.gather(chips, zones.filter(each => !Heat.gives(chips, each.part)),
                Heat.coreCpus(Heat.cpusOf(processor)), ProcessorTelemetry.reading?.temperatures),
            throttled: Heat.throttledOf(processor),
            zones: zones,
            cooling: Heat.coolingOf(heldOf("cooling"))
        };
    }

    function sample() {
        const now = Date.now();
        const kept = reads;
        if (due(kept, now)) {
            reads = { last: { at: now, said: sensed() }, before: kept ? kept.last : null };
            push(reads);
            if (!settled(reads))
                soon.restart();
        }
        return reads;
    }

    // Adds a read to every spark, starting them afresh after a gap: each
    // temperature and fan as it reads, and the throttling as how many
    // times a second it slowed since the read before.
    function push(kept) {
        const fresh = settled(kept);
        const added = (name, value) => (fresh ? series[name] ?? [] : []).concat([value])
            .slice(-graphed);
        const now = kept.last.said;
        const out = {};
        for (const part of now.parts.temps)
            out[tempKey(part)] = added(tempKey(part), part.value);
        for (const fan of now.parts.fans)
            out[fanKey(fan)] = added(fanKey(fan), fan.value);
        const before = kept.before ? kept.before.said.throttled : null;
        if (now.throttled && before) {
            const seconds = (kept.last.at - kept.before.at) / 1000;
            out.throttled = added("throttled", Math.max(0, now.throttled.times - before.times)
                / seconds);
        }
        series = out;
    }

    function tempKey(part) {
        return `${part.id}:${part.nth}`;
    }

    function fanKey(fan) {
        return `fan:${fan.id}:${fan.n}`;
    }

    // A part's name as the vocabulary says it, numbered when there are
    // two of it, as two disks are.
    function partName(part) {
        const name = Words.name(`heat:${part.id}`);
        return part.nth > 1 ? `${name} ${part.nth}` : name;
    }

    function fanName(fan) {
        return fan.id === "fan" ? Words.name("heat:fan", "", { n: fan.n })
            : partNamed("heat", fan.label);
    }

    // A disk's other sensor, by its label: "Sensor 2".
    function sensorName(label) {
        const n = /^Sensor (\d+)$/.exec(label);
        return n ? Words.name("heat:sensor", "", { n: n[1] })
            : label.charAt(0).toUpperCase() + label.slice(1);
    }

    // Its Last Reads, while shown, as a spark and its range.
    function lately(values, said, drawn) {
        const least = Math.min(...values);
        const most = Math.max(...values);
        const span = Math.round(least) === Math.round(most) ? said(most)
            : `${number(least)} to ${said(most)}`;
        return figure({
            name: Words.name("heat:lately"),
            side: span,
            graph: spark(drawn),
            note: values.length === 1 ? "one read so far, one a second while it's shown"
                : `the last ${values.length} reads, one a second while it's shown`
        });
    }

    function hottest(list) {
        return list.reduce((a, b) => (a && a.value >= b.value ? a : b), null);
    }

    // What's left to a limit, or how far past it, in Heat's words: "29 °
    // below high".
    function limitSaid(left) {
        const said = Words.of(`heat:${left.limit}`).said ?? {};
        return (said[left.past ? "past" : "below"] ?? "").replace("{by}", number(left.by));
    }

    // A figure to three significant digits in its unit, with the real
    // minus: "11.4 V".
    function electric(value, unit) {
        return `${Machine.digits(value).replace("-", "\u2212")} ${unit}`;
    }

    // The firmware's points that act on heat, in the order it acts.
    readonly property var trips: ["passive", "active", "hot", "critical"]

    // A part's drawer: where the firmware acts on its heat, how far it's
    // stepping the processor's cooling, what its chip says of electricity,
    // and last the identifiers it's read by, its chip and its zones, in the
    // Name face (VKCF3Z, 2026-11-08). None where it has none of these.
    function tempDrawer(part, said) {
        const facts = [];
        const tripped = Heat.tripsFor(said.zones, part.id)
            .filter(trip => provider.trips.includes(trip.type));
        for (const trip of tripped)
            facts.push({ label: Words.name(`heat:trip:${trip.type}`), value: degrees(trip.value) });
        if (part.id === "processor")
            for (const cooling of said.cooling)
                facts.push({
                    label: Words.name("heat:cooling"),
                    value: `${cooling.state} of ${cooling.max}`,
                    qualifier: cooling.count > 1 ? `${cooling.type} \u00d7${cooling.count}`
                        : cooling.type
                });
        const electrics = part.electric ?? { volts: [], amps: [], watts: [] };
        for (const [list, id, unit] of [[electrics.volts, "heat:voltage", "V"],
            [electrics.amps, "heat:current", "A"], [electrics.watts, "heat:drawing", "W"]])
            for (const each of list)
                facts.push({
                    label: each.label !== "" ? each.label
                        : list.length > 1 ? `${Words.name(id)} ${each.n}` : Words.name(id),
                    value: electric(each.value, unit)
                });
        if (part.chip !== "")
            facts.push({ label: Words.name("heat:chip"), value: part.chip, named: true });
        const zoned = tripped.map(trip => trip.zone)
            .filter((zone, at, all) => all.indexOf(zone) === at);
        if (zoned.length > 0)
            facts.push({ label: Words.name("heat:zone"), value: zoned.join(", "), named: true });
        return facts.length > 0 ? { title: partName(part), facts: facts } : undefined;
    }

    function tempRow(part, said) {
        const core = hottest(part.cores);
        const sensor = hottest(part.sensors);
        // Where a part of it reads hotter than the whole, as a core or a
        // disk's second sensor can, its note says so first.
        const hotter = core && Math.round(core.value) > Math.round(part.value)
            ? `the hottest, core ${core.cpu} at ${degrees(core.value)}`
            : sensor && Math.round(sensor.value) > Math.round(part.value)
            ? `${sensorName(sensor.label).toLowerCase()} hotter at ${degrees(sensor.value)}` : "";
        // Beside it, what's left to its limit, which alone wears the
        // warning past the high and the alert past the critical (VKCF3Z,
        // 2026-11-08).
        const left = Heat.leftTo(part.value, part.high, part.crit);
        const items = [{ said: degrees(part.value), keep: 3 }];
        if (left)
            items.push({ said: limitSaid(left), keep: 1, hue: left.hue });
        return holderRow({
            name: partName(part),
            side: degrees(part.value),
            sideItems: items,
            graph: level(part.value, part),
            glyph: Words.glyph("heat"),
            note: hotter,
            drawer: tempDrawer(part, said)
        }, () => tempParts(part));
    }

    function tempParts(part) {
        const values = series[tempKey(part)] ?? [part.value];
        const rows = [lately(values, degrees, Heat.spark(values))];
        if (part.cores.length > 0) {
            const core = hottest(part.cores);
            const levels = part.cores.map(each => Heat.level(each.value, part.high, part.crit,
                part.id));
            rows.push(holderRow({
                name: Words.name("heat:cores"),
                side: `${part.cores.length}`,
                graph: strip(levels.map(each => each.value), levels.some(each => each.warn)),
                glyph: Words.glyph("heat"),
                note: `the hottest, core ${core.cpu} at ${degrees(core.value)}`
            }, () => part.cores.map(each => figure({
                name: Words.name("heat:core", "", { n: each.cpu }),
                side: degrees(each.value),
                graph: level(each.value, part)
            }))));
        }
        for (const sensor of part.sensors)
            rows.push(figure({
                name: sensorName(sensor.label),
                side: degrees(sensor.value),
                graph: level(sensor.value, part)
            }));
        if (part.high !== null && (part.crit === null || part.high < part.crit))
            rows.push(figure({
                name: Words.name("heat:high"),
                side: degrees(part.high),
                note: "where it starts to warn"
            }));
        if (part.crit !== null)
            rows.push(figure({
                name: Words.name("heat:critical"),
                side: degrees(part.crit),
                note: "the most it's meant to reach"
            }));
        if (part.emergency !== null)
            rows.push(figure({
                name: Words.name("heat:emergency"),
                side: degrees(part.emergency),
                note: "where it shuts itself off"
            }));
        if (part.low !== null)
            rows.push(figure({
                name: Words.name("heat:low"),
                side: degrees(part.low),
                note: "the least it's meant to read"
            }));
        if (part.alarm)
            rows.push(figure({
                name: Words.name("heat:alarm"),
                side: (Words.of("heat:alarm").said ?? {}).raised ?? "",
                alerts: true,
                note: "its chip says a limit was crossed"
            }));
        return rows;
    }

    function throttledRow(said) {
        const drawn = series.throttled ?? [];
        return holderRow({
            name: Words.name("heat:throttled"),
            side: times(said.times),
            graph: spark(Machine.spark(drawn, 1)),
            glyph: Words.glyph("heat"),
            note: said.ms ? `for ${lasted(said.ms)} in all, since the machine started`
                : "since the machine started"
        }, () => throttledParts(said));
    }

    function throttledParts(said) {
        const most = Math.max(1, ...said.cores.map(each => each.count));
        const busiest = said.cores.reduce((a, b) => (a && a.count >= b.count ? a : b), null);
        const rows = [figure({
            name: Words.name("heat:package"),
            side: times(said.times),
            note: [said.ms ? `for ${lasted(said.ms)} in all` : "",
                said.longest ? `the longest ${lasted(said.longest)}` : ""]
                .filter(part => part !== "").join(", ")
        })];
        if (said.cores.length > 0)
            rows.push(holderRow({
                name: Words.name("heat:throttled-cores"),
                side: times(said.total),
                graph: strip(said.cores.map(each => each.count / most), false),
                glyph: Words.glyph("heat"),
                note: busiest && busiest.count > 0
                    ? `the most, core ${busiest.cpu}, ${times(busiest.count)}` : "each on its own"
            }, () => said.cores.map(each => figure({
                name: Words.name("heat:throttled-core", "", { n: each.cpu }),
                side: times(each.count),
                graph: bar(each.count / most, false)
            }))));
        return rows;
    }

    function fanRow(fan) {
        const name = fanName(fan);
        return holderRow({
            name: name,
            side: fan.value > 0 ? rpm(fan.value) : "stopped",
            graph: fan.max ? bar(fan.value / fan.max, false) : undefined,
            glyph: Theme.glyph.fan,
            note: [fan.max ? `rated for ${rpm(fan.max)}` : "",
                fan.power === null ? "" : fan.power >= 1 ? "at full power"
                : `at ${Math.round(fan.power * 100)}% power`]
                .filter(part => part !== "").join(", ")
        }, () => fanParts(fan));
    }

    function fanParts(fan) {
        const values = series[fanKey(fan)] ?? [fan.value];
        const rows = [lately(values, rpm, Heat.fanSpark(values))];
        if (fan.power !== null)
            rows.push(figure({
                name: Words.name("heat:power"),
                side: `${Math.round(fan.power * 100)}%`,
                graph: bar(fan.power, false),
                note: "how hard it's driven"
            }));
        if (fan.target !== null)
            rows.push(figure({
                name: Words.name("heat:aiming"),
                side: rpm(fan.target),
                note: "the speed it's asked for"
            }));
        if (fan.min !== null)
            rows.push(figure({
                name: Words.name("heat:least"),
                side: rpm(fan.min),
                note: "the slowest it turns while it turns"
            }));
        const control = fan.mode === null ? "" : (Words.of("heat:control").said ?? {})[fan.mode];
        if (control)
            rows.push(figure({
                name: Words.name("heat:control"),
                side: control,
                note: "what sets its speed"
            }));
        return rows;
    }

    function told() {
        const now = sample().last.said;
        const temps = now.parts.temps;
        const fans = now.parts.fans;
        if (temps.length === 0 && fans.length === 0)
            return null;
        const rows = [];
        for (const part of temps) {
            rows.push(tempRow(part, now));
            if (part.id === "processor" && now.throttled)
                rows.push(throttledRow(now.throttled));
        }
        if (now.throttled && !temps.some(part => part.id === "processor"))
            rows.push(throttledRow(now.throttled));
        for (const fan of fans)
            rows.push(fanRow(fan));
        const hot = hottest(temps);
        const turning = fans.filter(fan => fan.value > 0).length;
        const summary = [];
        if (hot)
            summary.push(`${degrees(hot.value)} at the ${Heat.phrase(partName(hot))}`);
        if (fans.length > 0)
            summary.push(turning === 0 ? "fans stopped" : turning === 1 ? "a fan turning"
                : `${turning} fans turning`);
        return {
            headline: summary.join(", "),
            graph: hot ? level(hot.value, hot) : undefined,
            glyph: Words.glyph("heat"),
            rows: rows,
            note: "read just now",
            live: provider
        };
    }

    // One part as an answer, by its id: its figure as the headline, its
    // own parts a step in, and Return copying nothing, as Machine's do.
    function alone(id) {
        const whole = told();
        const name = Words.name(id);
        const part = whole ? whole.rows.find(row => row.name === name) : null;
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

    // While one of its answers is shown, it's asked again each second, so
    // its figures and sparks move; and once a moment after a first read,
    // so a spark has two reads as soon as it can.
    readonly property bool shown: launcher !== null && launcher.open && launcher.held.length === 0
        && launcher.answer?.live === provider
    // The processor's telemetry is watched for as long as its answer is up,
    // stepped into or not, so a part is there however soon it's opened.
    // Stepped in, the answer is held as a step rather than being the
    // launcher's answer, so the held steps are looked at too, as
    // Machine.qml's are.
    readonly property bool watched: launcher !== null && launcher.open
        && (launcher.answer?.live === provider
            || launcher.held.some(step => step.row?.live === provider))
    onWatchedChanged: {
        ProcessorTelemetry.watching += watched ? 1 : -1;
        if (!watched)
            ownHeard = false;
    }
    // Whether its first reading has come since the answer went up: that
    // one is read at once, stepped into or not, so the processor's own
    // parts are listed a moment after it's asked, and after it they move
    // with the rest.
    property bool ownHeard: false

    Connections {
        target: ProcessorTelemetry
        function onReadingChanged() {
            if (!provider.watched || provider.ownHeard || ProcessorTelemetry.reading === null)
                return;
            provider.ownHeard = true;
            provider.reads = null;
            provider.answered();
        }
    }

    // The machine daemon's chips, zones, cooling and processor, held once
    // Heat has been asked since the launcher opened, as Machine's place
    // asks it for its Heat row, until the launcher closes.
    // Whether the daemon has said anything since the launcher opened: its
    // first records are answered at once, shown or not, so Machine's Heat
    // row is there as soon as they are, and a read made before them is
    // let go; after that, only a shown answer moves with each.
    property bool heardSinceOpen: false

    Connections {
        target: daemon

        function onHeard(keys) {
            if (!provider.heardSinceOpen) {
                provider.heardSinceOpen = true;
                provider.reads = null;
                provider.answered();
            } else if (provider.shown) {
                provider.answered();
            }
        }
    }

    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open) {
                provider.heardSinceOpen = false;
                provider.askedSinceOpen = false;
            }
        }
    }

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/machine.sock`
        kind: ["sensor", "zone", "cooling", "processor"]
        live: provider.launcher !== null && provider.launcher.open && provider.askedSinceOpen
        keptThroughGap: false
        answersWhole: true
    }

    Timer {
        id: soon
        interval: provider.soonest
        onTriggered: {
            if (provider.shown)
                provider.answered();
        }
    }
}
