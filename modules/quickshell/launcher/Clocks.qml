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
// The time here goes on to how the clock is kept, as timekeeping last
// heard systemd-timesyncd say (timekeeping/src/main.rs): whether it's
// synchronised, the server, how far the clock is from it, the round trip's
// delay and jitter, the server's stratum and what it keeps its time by, how
// often it's asked and the leap second announced, each a figure Return
// copies. The file is watched, never asked, and says nothing on a machine
// that keeps no time that way.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Zones.js" as Zones

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

    function answer(typed) {
        // Cheap enough for every key, so the zones are read only for a
        // question of time.
        if (!Zones.asked(typed))
            return null;
        const zones = loaded();
        const now = Date.now();
        const local = -new Date(now).getTimezoneOffset();
        // The clock's own zone, which follows the laptop, before the one
        // the machine was built with.
        const here = Location.clockZone || (settings.zone ?? null);
        let told = Zones.answer(typed, zones, now, here, local);
        if (!told)
            return null;
        // Where the answer came from, in Joel's words rather than the
        // list's own name.
        let note = "the time zones kept on this machine";
        if (told.unknown !== undefined) {
            const asked = `\u201c${told.unknown}\u201d`;
            if (settings.lookups === false)
                return plain("where?", `no time zone kept here is ${asked}`);
            const found = ask("https://geocoding-api.open-meteo.com/v1/search?count=1&name="
                + encodeURIComponent(told.unknown), 24 * 60 * 60 * 1000);
            if (!found.value)
                return found.failed ? plain("no time", "Open-Meteo didn't answer")
                    : plain("\u2026", "asking Open-Meteo");
            const place = (found.value.results ?? [])[0];
            const zone = place ? zones.zones[place.timezone] : null;
            if (!zone)
                return plain("no such place", `Open-Meteo knows no ${asked}`);
            const mine = here ? zones.zones[here] ?? null : null;
            told = Zones.tell(zone, now, mine, local, place.name);
            const where = [place.name, place.country].filter(part => part).join(", ");
            note = `${where} \u00b7 Open-Meteo`;
        }
        // Only the time here is the clock's own, so only it says the clock
        // followed the laptop, with Put It Back first inside, which puts it
        // in the zone it kept before and holds it there, and only it says
        // how the clock here is kept.
        const ours = told.unknown === undefined && Zones.asked(typed).place === "";
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
        return {
            headline: told.headline,
            copy: told.copy,
            rows: back.concat(told.rows.map(row => copyRow({
                name: Words.name(`time:${row.part}`),
                side: row.text,
                note: row.note,
                copy: row.copy,
                glyph: Theme.glyph[row.glyph]
            })).concat(ours ? keeping() : [])),
            note: moved ? Location.movedSaid : note
        };
    }

    // How timesyncd keeps the clock, as timekeeping last wrote it, or null
    // for nothing said.
    property var kept: null

    // Milliseconds from microseconds, to a tenth.
    function millis(micros, signed) {
        const said = `${(Math.abs(micros) / 1000).toFixed(1)} ms`;
        return !signed ? said : `${micros < 0 ? "\u2212" : "+"}${said}`;
    }

    // A poll's seconds as minutes and seconds, as timedatectl says them.
    function every(seconds) {
        const minutes = Math.floor(seconds / 60);
        const left = seconds % 60;
        if (minutes === 0)
            return `every ${left} s`;
        return left === 0 ? `every ${minutes} min` : `every ${minutes} min ${left} s`;
    }

    // The clock's keeping as figures, in the order timedatectl says them.
    // Each is a figure, so each wears no mark.
    function keeping() {
        if (!kept)
            return [];
        const figure = (part, side, copy) => copyRow({
            name: Words.name(`time:${part}`),
            side: side,
            copy: copy ?? side,
            glyph: ""
        });
        const server = [kept.server, kept.address].filter(part => part).join(" \u00b7 ");
        return [
            figure("synchronised", kept.synchronised ? "yes" : "no"),
            server !== "" ? figure("server", server, kept.server || kept.address) : null,
            figure("offset", millis(kept.offset, true), `${kept.offset} \u00b5s`),
            figure("delay", millis(kept.delay, false), `${kept.delay} \u00b5s`),
            figure("jitter", millis(kept.jitter, false), `${kept.jitter} \u00b5s`),
            figure("stratum", String(kept.stratum)),
            kept.reference ? figure("reference", kept.reference) : null,
            figure("poll", every(kept.poll)),
            figure("leap", kept.leap)
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
                provider.kept = said.version === 1 ? said : null;
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
