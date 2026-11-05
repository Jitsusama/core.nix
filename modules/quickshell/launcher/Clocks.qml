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
// The time here goes on to how the clock is kept, as timekeeping last
// heard systemd-timesyncd say (timekeeping/src/main.rs): whether it's
// synchronised and since when, then the server it asks, a thing Return
// steps into for all it says (its address, stratum and reference, the
// offset, delay and jitter, its root delay and dispersion, precision, leap
// second and NTP version, the last sync, the poll and its bounds, the
// packets and the clock's frequency), its drawer saying them all; then
// the service keeping it, and the other servers it's given (VKCF3Z's
// design, 2026-10-29, for Joel's "for time i want to be able to dig into
// ntp server details"). Each figure is one Return copies. The file is
// watched, never asked; with nothing keeping the time that way, Time says
// so and no more.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Zones.js" as Zones
import "Ago.js" as Ago
import "Moment.js" as Moment
import "Numbers.js" as Numbers

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

    // How timesyncd keeps the clock, as timekeeping last wrote it, or null
    // while nothing keeps it that way.
    property var kept: null

    // Milliseconds from microseconds, to a tenth.
    function millis(micros, signed) {
        const said = `${(Math.abs(micros) / 1000).toFixed(1)} ms`;
        return !signed ? said : `${micros < 0 ? "\u2212" : "+"}${said}`;
    }

    // A poll's seconds as minutes and seconds, as timedatectl says them.
    function span(seconds) {
        const minutes = Math.floor(seconds / 60);
        const left = seconds % 60;
        if (minutes === 0)
            return `${left} s`;
        return left === 0 ? `${minutes} min` : `${minutes} min ${left} s`;
    }

    // A precision, a power of two of a second, in the unit that reads it.
    function finest(power) {
        const seconds = Math.pow(2, power);
        if (seconds < 1e-6)
            return `${Math.round(seconds * 1e9)} ns`;
        return seconds < 1e-3 ? `${Math.round(seconds * 1e6)} \u00b5s`
            : `${(seconds * 1000).toFixed(1)} ms`;
    }

    // What Time says of its keeping, in the vocabulary's words.
    function keptWord(key) {
        return String((Words.of("time:server").said ?? {})[key] ?? "");
    }

    // What the server says, each a part named and its figure, in the order
    // its inside lists them; the drawer shows the same, so both read alike.
    function serverParts() {
        const at = kept?.exchange;
        if (!at)
            return [];
        const part = (id, side, copy, note) => ({ id: id, side: side, copy: copy ?? side,
            note: note ?? "" });
        const micros = value => `${value} \u00b5s`;
        const lastPoll = keptWord("asOfLastPoll");
        const poll = kept.poll ?? {};
        return [
            part("name", at.server),
            part("address", at.address),
            part("stratum", String(at.stratum)),
            part("reference", at.reference),
            part("offset", millis(at.offset, true), micros(at.offset), lastPoll),
            part("delay", millis(at.delay, false), micros(at.delay)),
            part("jitter", millis(at.jitter, false), micros(at.jitter), lastPoll),
            part("root-delay", millis(at.rootDelay, false), micros(at.rootDelay)),
            part("root-dispersion", millis(at.rootDispersion, false), micros(at.rootDispersion)),
            part("precision", finest(at.precision), `2^${at.precision} s`),
            part("leap", at.leap),
            part("ntp-version", String(at.ntpVersion)),
            part("last-sync", Ago.ago(at.last, Date.now()), new Date(at.last).toISOString()),
            part("poll", `every ${span(poll.now ?? 0)}`, `${poll.now ?? 0} s`,
                keptWord("between").replace("{least}", span(poll.least ?? 0))
                    .replace("{most}", span(poll.most ?? 0))),
            part("packets", String(at.packets)),
            part("frequency", `${kept.frequency < 0 ? "\u2212" : "+"}`
                + `${Math.abs(kept.frequency).toFixed(3)} ppm`)
        ].filter(each => each.side !== "");
    }

    // The other servers timesyncd is given, each where it's given from,
    // but for the one it asks now.
    function otherServers() {
        const current = kept?.exchange?.server ?? "";
        const from = kept?.servers ?? {};
        return ["system", "link", "runtime", "fallback"].reduce((listed, source) => listed.concat(
            (from[source] ?? []).filter(name => name !== current
                && !listed.some(one => one.name === name))
                .map(name => ({ name: name, note: keptWord(source) }))), []);
    }

    // The clock's keeping: whether it's synchronised, the server, the
    // service and the other servers, in that order (VKCF3Z). A figure wears
    // no mark; the server, holding more, wears its mark dimmed.
    function keeping() {
        const figure = (part, side, copy, note) => copyRow({
            name: Words.name(`time:${part}`),
            side: side,
            copy: copy ?? side,
            note: note ?? "",
            glyph: ""
        });
        if (!kept)
            return [figure("synchronised", keptWord("nothing"))];
        const at = kept.exchange ?? null;
        const synced = !kept.synchronised ? keptWord("no")
            : at ? `${keptWord("yes")}, ${Ago.ago(at.last, Date.now())}` : keptWord("yes");
        const parts = serverParts();
        const server = at ? thingRow({
            key: "time:server",
            name: at.server || at.address,
            side: [keptWord("stratum").replace("{stratum}", at.stratum), millis(at.offset, true)]
                .join(", "),
            glyph: Theme.glyph.clock,
            // The server is a thing, one Joel digs into, so its drawer is
            // out beside it.
            thing: true,
            drawer: () => ({
                title: at.server || at.address,
                facts: parts.map(each => ({ label: Words.name(`time:server:${each.id}`),
                    value: each.side }))
            })
        }, () => parts.map(each => figure(`server:${each.id}`, each.side, each.copy,
            each.note))) : null;
        const others = otherServers();
        return [
            figure("synchronised", synced),
            server,
            figure("service", "systemd-timesyncd", "systemd-timesyncd", keptWord("running")),
            others.length > 0 ? holderRow({
                key: "time:servers",
                name: Words.name("time:servers"),
                side: String(others.length),
                glyph: Theme.glyph.clock
            }, () => others.map(one => copyRow({ name: one.name, note: one.note, glyph: "" })))
                : null
        ].filter(row => row !== null);
    }

    FileView {
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/timekeeping.json`
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: {
            try {
                const said = JSON.parse(text());
                provider.kept = said.version === 2 ? said : null;
            } catch (e) {
                provider.kept = null;
            }
        }
        onLoadFailed: provider.kept = null
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
