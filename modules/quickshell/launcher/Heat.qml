// How hot the machine runs, answered in the field by Heat's words
// ("temperature", "heat", "hot", "thermal", "sensors" or "fans", as
// words.json lists them). The headline is the hottest part and the fans;
// the rows are each part by its name first, then what it reads: the
// Processor, how often it's Throttled, its NPU, Image Processor, Media
// Engine and Display Engine, the Disk, the Memory, the Wi-Fi card, the
// Battery, the Air inside and each fan. Each temperature is drawn
// against the most its chip says it may reach, warning near it, and holds,
// a step in, what it's read Lately while shown, its other sensors (a
// disk's second, hottest one), the processor's Cores, and its High and
// Critical points. A fan is drawn against what it's rated for and holds
// its history, its Power and what it's Aiming for. Throttled counts the
// times the processor slowed itself to cool down, as the whole processor
// and core by core.
//
// The readings are the kernel's hwmon files (Sensors.qml, Heat.js), read
// when asked and once a second while the answer is shown, never for a key,
// the same way the machine's answers read the processor and the disk. The
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
import QtQuick
import "Heat.js" as Heat
import "Machine.js" as Machine
import "Numbers.js" as Numbers

Provider {
    id: provider

    examples: ["temperature", "fans"]

    // Sensors is made at its first use, and lists the machine's chips a
    // moment after; asked first as Machine is stepped into, it would have
    // none yet, and Machine, made once, would list no Heat (15J7N3,
    // 2026-10-29). So it's made as Heat is, with the shell, as each of
    // Heat's and Machine's own was before they shared one.
    Component.onCompleted: Sensors.chips()

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
        if (part)
            return alone(part);
        if (asks(typed) === "")
            return null;
        return told();
    }

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

    // Everything Heat says, read now: each chip's temperatures (of a chip
    // measuring many, only those it can name) and fans, a thermal zone
    // for a part no chip gives, and the throttling counts.
    function sensed() {
        const chips = Sensors.chips().map(chip => ({
            part: chip.part,
            temps: Sensors.readings(chip, "temp",
                chip.part === "" ? label => Heat.labelled(label) !== "" : null).map(s => {
                const said = Sensors.degrees(chip, s);
                return {
                    n: s.n,
                    label: s.label,
                    value: said.value,
                    high: said.high,
                    crit: said.most
                };
            }),
            fans: Sensors.readings(chip, "fan").map(s => Object.assign({
                n: s.n,
                label: s.label,
                value: s.value
            }, Sensors.fan(chip, s)))
        }));
        const zones = Sensors.zones()
            .map(zone => ({ part: Heat.zonePart(zone.type), zone: zone }))
            .filter(each => each.part !== "" && !Heat.gives(chips, each.part))
            .map(each => ({ part: each.part, value: Sensors.zoneDegrees(each.zone) }))
            .filter(each => each.value !== null);
        return {
            parts: Heat.gather(chips, zones, Heat.coreCpus(Sensors.cpus()),
                ProcessorTelemetry.reading?.temperatures),
            throttled: Heat.throttled(Sensors.throttles())
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

    // What it's read Lately, while shown, as a spark and its range.
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

    function tempRow(part) {
        const core = hottest(part.cores);
        const sensor = hottest(part.sensors);
        // Where a part of it reads hotter than the whole, as a core or a
        // disk's second sensor can, its note says so first.
        const hotter = core && Math.round(core.value) > Math.round(part.value)
            ? `the hottest, core ${core.cpu} at ${degrees(core.value)}`
            : sensor && Math.round(sensor.value) > Math.round(part.value)
            ? `${sensorName(sensor.label).toLowerCase()} hotter at ${degrees(sensor.value)}` : "";
        return holderRow({
            name: partName(part),
            side: degrees(part.value),
            graph: level(part.value, part),
            glyph: Words.glyph("heat"),
            note: [hotter, part.crit !== null ? `at most ${degrees(part.crit)}` : ""]
                .filter(said => said !== "").join("; ")
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
            side: fan.value > 0 ? rpm(fan.value) : "still",
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
            rows.push(tempRow(part));
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
            summary.push(turning === 0 ? "fans still" : turning === 1 ? "a fan turning"
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
    readonly property bool watched: launcher !== null && launcher.open
        && launcher.answer?.live === provider
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
}
