// The time here or somewhere else, answered in the field: "time", "time in
// tokyo", "london time", with the date there, how far it is from here and
// its zone as parts, each named first. Zones.js tells it from tzdata's
// zones, built at build time
// into `list` and read a moment after the shell starts. A place tzdata
// doesn't name is found by Open-Meteo, only when it's asked for as "time
// in" it, and never with `lookups` off. The time here is in the zone the
// clock keeps (Location.qml), or `zone`, the machine's own, before that's
// read. Once the clock has followed the laptop into another zone, the time
// here says so, and Put It Back leads what's inside (Joel, 2026-10-28: "i
// really want my timezone to automatically change if possible").
//
// The time here holds the clock and never the date, which is Date's
// (VKCF3Z's ruling, 2026-10-30): its Zone, a thing saying its offset, its
// abbreviation, its next change and tzdata's version; then UTC, Unix
// time, TAI, GPS time and sidereal time, each to the second and counting
// while it's shown or stepped into, reckoned as Moment.js says. "utc",
// "epoch", "tai" and the rest of Time's words ask it whole.
//
// The time here goes on to how chrony keeps the clock, as timekeeping
// reads it from chronyd (Timekeeping.qml, timekeeping/src/main.rs): Clock,
// stepping into its offset, frequency, skew, root delay and dispersion,
// leap second, last sync, next poll, the hardware clock and its drift and
// the service; then Servers, every server chrony asks, the chosen first,
// each a thing whose side is its offset, whose note says where it stands,
// and whose drawer and inside give every figure chronyd has of it and why
// it was chosen or set aside (VKCF3Z's design, for Joel's "for time i want
// to be able to dig into ntp server details"). Keeping.js words each
// figure, and each is one Return copies. It's asked only while home or the
// time here is shown; with nothing keeping the time that way, Clock says
// so and no more.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Zones.js" as Zones
import "Ago.js" as Ago
import "Moment.js" as Moment
import "Numbers.js" as Numbers
import "Keeping.js" as Keeping

Provider {
    id: provider

    examples: ["time in tokyo", "london time"]

    property var table: null

    function loaded() {
        if (table === null) {
            table = { zones: {}, named: {} };
            if (settings.list) {
                file.path = settings.list;
                table = Zones.prepare(read(file));
            }
        }
        return table;
    }

    // An answer that tells no time, only why.
    function plain(headline, note) {
        return { headline: headline, copy: "", rows: [], note: note };
    }

    // An answer that came to nothing, saying why in a sentence, once: the
    // launcher shows it as a row of its own, as quietly as nothing found.
    function missed(why) {
        return { headline: why, copy: "", rows: [], missed: true };
    }

    // Time's own words beside "time", each asking the time here whole.
    readonly property var partWords: Words.answers("time")

    function answer(typed) {
        // Cheap enough for every key, so the zones are read only for a
        // question of time.
        const part = partWords.includes(plainly(typed));
        if (!part && !Zones.asked(typed))
            return null;
        const asked = part ? "time" : typed;
        const zones = loaded();
        const now = Date.now();
        const local = -new Date(now).getTimezoneOffset();
        // The clock's own zone, which follows the laptop, before the one
        // the machine was built with.
        const here = Location.clockZone || (settings.zone ?? null);
        let told = Zones.answer(asked, zones, now, here, local);
        if (!told)
            return null;
        // Where the answer came from, in Joel's words rather than the
        // list's own name.
        let note = "the time zones kept on this machine";
        if (told.unknown !== undefined) {
            const asked = `\u201c${told.unknown}\u201d`;
            if (settings.lookups === false)
                return missed(`No time zone kept here is ${asked}.`);
            const found = ask("https://geocoding-api.open-meteo.com/v1/search?count=1&name="
                + encodeURIComponent(told.unknown), 24 * 60 * 60 * 1000);
            if (!found.value)
                return found.failed ? missed("Open-Meteo didn't answer.")
                    : plain("\u2026", "asking Open-Meteo");
            const place = (found.value.results ?? [])[0];
            const zone = place ? zones.zones[place.timezone] : null;
            if (!zone)
                return missed(`Open-Meteo knows no ${asked}.`);
            const mine = here ? zones.zones[here] ?? null : null;
            told = Zones.tell(zone, now, mine, local, place.name);
            const where = [place.name, place.country].filter(part => part).join(", ");
            note = `${where} \u00b7 Open-Meteo`;
        }
        // Only the time here is the clock's own, so only it says the clock
        // followed the laptop, with Put It Back first inside, which puts it
        // in the zone it kept before and holds it there, and only it says
        // how the clock here is kept.
        const ours = told.unknown === undefined && Zones.asked(asked).place === "";
        const moved = ours && Location.clockMoved;
        const back = moved ? [thing({
            key: "verb:time:put-back",
            verb: Words.verb("verb:time:put-back"),
            name: Words.name("verb:time:put-back"),
            glyph: Words.glyph("verb:time:put-back"),
            side: Location.city(Location.moved.from),
            hue: "action",
            stays: true,
            run: () => Location.putBack()
        })] : [];
        const parts = ours ? [zoneThing(told, zones.zones[here] ?? null, now)].concat(figures(now))
            : told.rows.map(row => copyRow({
                name: Words.name(`time:${row.part}`),
                side: row.text,
                note: row.note,
                copy: row.copy,
                glyph: Theme.glyph[row.glyph]
            }));
        return {
            headline: told.headline,
            copy: told.copy,
            // Its place's glyph, as Now's rows each wear theirs, rather
            // than its first row's.
            glyph: Theme.glyph.clock,
            rows: back.concat(parts, ours ? keeping() : []),
            note: moved ? Location.movedSaid : note,
            // The time here counts, so it's asked again each second while
            // it's shown or stepped into.
            live: ours ? provider : null
        };
    }

    // A part of the time here in the vocabulary's words.
    function timeWord(id, key) {
        return String((Words.of(id).said ?? {})[key] ?? "");
    }

    // The zone the clock keeps, a thing: its offset and abbreviation, when
    // it next changes and to what, and which tzdata told it, each a figure
    // a step in and a fact in its drawer.
    function zoneThing(told, zone, now) {
        const said = told.rows.find(row => row.part === "zone") ?? null;
        const minutes = -new Date(now).getTimezoneOffset();
        const standing = zone ? Zones.standing(zone, Math.floor(now / 1000)) : [minutes, ""];
        const next = zone ? Zones.nextChange(zone, Math.floor(now / 1000)) : null;
        const abbreviation = /^[A-Za-z]/.test(standing[1]) ? standing[1] : "";
        const parts = [
            { id: "offset", side: Zones.offset(standing[0]), note: "" },
            { id: "abbreviation", side: abbreviation, note: "" },
            { id: "next", side: next ? Moment.dayAndClock(next.when, now)
                : timeWord("time:zone", "never"),
                note: next ? [/^[A-Za-z]/.test(next.abbreviation) ? next.abbreviation : "",
                    Zones.offset(next.minutes)].filter(one => one !== "").join(", ") : "" },
            { id: "database", side: settings.tzdata ?? "", note: timeWord("time:zone", "iana") }
        ].filter(part => part.side !== "");
        const name = said ? said.text : Zones.offset(standing[0]);
        return thingRow({
            key: "time:zone",
            name: Words.name("time:zone"),
            side: name,
            note: said ? said.note : "",
            glyph: Theme.glyph.zone,
            thing: true,
            drawer: () => ({
                eyebrow: Words.name("time:zone:plate"),
                title: name,
                facts: parts.map(part => ({ label: Words.name(`time:zone:${part.id}`),
                    value: part.side }))
            })
        }, () => parts.map(part => copyRow({
            name: Words.name(`time:zone:${part.id}`),
            side: part.side,
            copy: part.side,
            note: part.note,
            glyph: ""
        })));
    }

    // The time as other clocks keep it, each to the second, a figure that
    // copies what it shows.
    function figures(now) {
        const figure = (id, side, note) => copyRow({
            name: Words.name(`time:${id}`),
            side: side,
            copy: side,
            note: note ?? "",
            glyph: ""
        });
        const gps = Moment.gps(now);
        const longitude = Location.here?.longitude;
        const known = typeof longitude === "number";
        const degrees = known ? `${Numbers.format(Math.abs(longitude), 2, "", ".")}\u00b0 `
            + (longitude < 0 ? "W" : "E") : "";
        return [
            figure("utc", Moment.utcClock(now)),
            figure("unix", String(Moment.unix(now)), timeWord("time:unix", "since")),
            figure("tai", Moment.tai(now), timeWord("time:tai", "ahead")),
            figure("gps", timeWord("time:gps", "week").replace("{week}", String(gps.week))
                .replace("{seconds}", Numbers.format(gps.seconds, 0, ",", ".")),
                timeWord("time:gps", "ahead")),
            figure("sidereal", Moment.sidereal(now, known ? longitude : 0),
                known ? timeWord("time:sidereal", "local").replace("{longitude}", degrees)
                    : timeWord("time:sidereal", "greenwich"))
        ];
    }

    // While the time here is shown, as the answer or a step held in it,
    // it's asked again each second, so its figures count; never for a key.
    readonly property bool shown: launcher !== null && launcher.open
        && (launcher.answer?.live === provider
            || (launcher.held ?? []).some(step => step.row?.live === provider))

    Timer {
        interval: 1000
        repeat: true
        running: provider.shown
        onTriggered: provider.answered()
    }

    // While home or the time here is shown, chrony's keeping is watched,
    // so stepping into Clock or Servers finds it there; never while the
    // launcher is closed or elsewhere.
    readonly property bool keepingWatched: launcher !== null && launcher.open
        && (launcher.atHome === true || shown)
    onKeepingWatchedChanged: Timekeeping.watching += keepingWatched ? 1 : -1

    // How chrony keeps the clock, as timekeeping last said, or null while
    // it hasn't, or nothing keeps it that way.
    readonly property var kept: Timekeeping.reading

    // What Time says of its keeping, in the vocabulary's words: the
    // clock's and the servers'.
    readonly property var clockSaid: Words.of("time:clock").said ?? {}
    readonly property var serversSaid: Words.of("time:servers").said ?? {}

    // A figure of the keeping, a row that copies what it shows.
    function figure(id, side, note) {
        return copyRow({
            name: Words.name(id),
            side: side,
            copy: side,
            note: note ?? "",
            glyph: ""
        });
    }

    // The clock's figures, each a part named and its figure, in the order
    // Clock's inside lists them (VKCF3Z): how far it's out and how fast it
    // runs, the skew, root delay and dispersion its reference gives, the
    // leap second, the last sync and the next poll, the hardware clock and
    // its drift, and the service keeping it.
    function clockParts(now) {
        const clock = kept.clock;
        const said = clockSaid;
        const chosen = kept.servers.find(server => server.standing === "chosen") ?? null;
        const hardware = kept.hardwareClock ?? null;
        const part = (id, side, note) => ({ id: id, side: side, note: note ?? "" });
        // chronyd's correction is how far the clock is behind true time.
        return [
            part("offset", Keeping.out(-clock.correction, said), said.lastOffset
                .replace("{offset}", Keeping.seconds(clock.lastOffset, true))),
            part("frequency", Keeping.runs(clock.frequency, said), said.residual
                .replace("{ppm}", Keeping.ppm(clock.residualFrequency, true))),
            part("skew", Keeping.ppm(clock.skew, false)),
            part("root-delay", Keeping.seconds(clock.rootDelay, false)),
            part("root-dispersion", Keeping.seconds(clock.rootDispersion, false)),
            part("leap", Keeping.leap(clock.leap, said)),
            part("last-sync", clock.lastSync ? Ago.ago(clock.lastSync, now) : said.never,
                clock.reference ? said.by.replace("{server}", clock.reference) : ""),
            part("next-poll", chosen ? said.next.replace("{span}",
                Keeping.span(Keeping.nextPoll(chosen))) : "",
                chosen ? said.every.replace("{span}", Keeping.span(chosen.poll)) : ""),
            part("hardware", hardware ? Keeping.out(hardware.fast, said) : ""),
            part("drift", hardware ? Keeping.daily(hardware.gain, said) : "",
                hardware ? Keeping.ppm(hardware.gain, true) : ""),
            part("service", settings.chrony ? `chrony ${settings.chrony}` : "chrony",
                said.running)
        ].filter(each => each.side !== "");
    }

    // What a server says, each a part named and its figure, in the order
    // its inside lists them; its drawer shows the same, so both read alike.
    function serverParts(server) {
        const said = serversSaid;
        const samples = server.samples ?? null;
        const exchange = server.exchange ?? null;
        const chosen = server.chosen ?? null;
        const part = (id, side, note) => ({ id: id, side: side, note: note ?? "" });
        const exchanged = exchange ? [
            part("reference", exchange.reference),
            part("ntp-version", String(exchange.version)),
            part("precision", Keeping.seconds(exchange.precision, false)),
            part("root-delay", Keeping.seconds(exchange.rootDelay, false)),
            part("root-dispersion", Keeping.seconds(exchange.rootDispersion, false)),
            part("delay", Keeping.seconds(exchange.delay, false)),
            part("dispersion", Keeping.seconds(exchange.dispersion, false)),
            part("response", Keeping.seconds(exchange.responseTime, false)),
            part("tests", said.tests.replace("{passed}", String(exchange.testsPassed))),
            part("packets", said.packets.replace("{sent}", String(exchange.sent))
                .replace("{received}", String(exchange.received))
                .replace("{good}", String(exchange.good)))
        ] : [];
        const sampled = samples ? [
            part("samples", said.samples.replace("{samples}", String(samples.count))
                .replace("{span}", Keeping.span(samples.span))),
            part("estimate", Keeping.seconds(samples.offset, true), said.within
                .replace("{error}", Keeping.seconds(samples.offsetError, false))),
            part("deviation", Keeping.seconds(samples.deviation, false)),
            part("frequency", Keeping.ppm(samples.residualFrequency, true)),
            part("skew", Keeping.ppm(samples.skew, false))
        ] : [];
        return [
            part("address", server.address !== server.name ? server.address : ""),
            part("why", Keeping.why(server, said)),
            part("stratum", server.stratum > 0 ? String(server.stratum) : ""),
            // Its name already says it's an interval, so the span stands alone.
            part("poll", Keeping.span(server.poll)),
            part("reach", Keeping.reach(server.reach, said)),
            part("last-sample", server.lastSample !== null && server.lastSample !== undefined
                ? said.ago.replace("{span}", Keeping.span(server.lastSample)) : ""),
            part("offset", server.lastSample !== null && server.lastSample !== undefined
                ? Keeping.seconds(server.offset, true) : "", said.within
                    .replace("{error}", Keeping.seconds(server.error, false))),
            part("interval", chosen && chosen.high > chosen.low ? said.interval
                .replace("{low}", Keeping.seconds(chosen.low, true))
                .replace("{high}", Keeping.seconds(chosen.high, true)) : "")
        ].concat(sampled, exchanged).filter(each => each.side !== "");
    }

    // One server chrony asks, a thing: its side its last offset, its note
    // where it stands, its drawer and its inside every figure (VKCF3Z).
    function serverThing(server) {
        const parts = serverParts(server);
        const name = server.name || server.address;
        const sampled = server.lastSample !== null && server.lastSample !== undefined;
        // Known by its address, since the launcher keeps a selection by its
        // row's key as the reading changes; what it is says it's a server.
        return thingRow({
            key: `time-server:${server.address}`,
            is: "time:server",
            name: name,
            side: sampled ? Keeping.seconds(server.offset, true) : "",
            note: Keeping.standing(server, serversSaid),
            glyph: Theme.glyph.clock,
            thing: true,
            drawer: () => ({
                eyebrow: Words.name("time:server"),
                title: name,
                facts: parts.map(each => ({ label: Words.name(`time:server:${each.id}`),
                    value: each.side }))
            })
        }, () => parts.map(each => figure(`time:server:${each.id}`, each.side, each.note)));
    }

    // The clock's keeping: Clock, stepping into its figures, and Servers,
    // stepping into every server chrony asks, the chosen first (VKCF3Z).
    // With nothing keeping the time, Clock says so and no more.
    function keeping() {
        const said = clockSaid;
        if (!kept) {
            const waiting = Timekeeping.watching > 0 && !Timekeeping.refused;
            return [figure("time:clock", waiting ? "\u2026" : said.nothing)];
        }
        const now = Date.now();
        const parts = clockParts(now);
        const servers = kept.servers;
        const out = servers.filter(server => Keeping.answered(server.reach) === 0).length;
        return [
            thingRow({
                key: "time:clock",
                name: Words.name("time:clock"),
                side: kept.synchronised ? said.synchronised : said.unsynchronised,
                glyph: Theme.glyph.clock
            }, () => parts.map(each => figure(`time:clock:${each.id}`, each.side, each.note))),
            servers.length > 0 ? holderRow({
                key: "time:servers",
                name: Words.name("time:servers"),
                side: String(servers.length),
                note: out > 0 ? serversSaid.outOfReach.replace("{count}", String(out)) : "",
                glyph: Theme.glyph.clock
            }, () => servers.map(server => serverThing(server))) : null
        ].filter(row => row !== null);
    }

    // Read a moment after the shell starts, so the first question doesn't
    // wait on it.
    Timer {
        interval: 2000
        running: true
        onTriggered: provider.loaded()
    }

    FileView {
        id: file
        blockLoading: true
        printErrors: false
    }
}
