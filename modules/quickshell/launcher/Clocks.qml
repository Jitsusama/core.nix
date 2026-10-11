// The time here or somewhere else, answered in the field: "time", "time in
// tokyo", "london time", with the day there, how far it is from here and its
// zone as rows. Zones.js tells it from tzdata's zones, built at build time
// into `list` and read a moment after the shell starts. A place tzdata
// doesn't name is found by Open-Meteo, only when it's asked for as "time
// in" it, and never with `lookups` off. `zone` is the machine's own.
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
        const here = settings.zone ?? null;
        let told = Zones.answer(typed, zones, now, here, local);
        if (!told)
            return null;
        let note = "tzdata";
        if (told.unknown !== undefined) {
            const asked = `\u201c${told.unknown}\u201d`;
            if (settings.lookups === false)
                return plain("where?", `tzdata names no ${asked}`);
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
        return {
            headline: told.headline,
            copy: told.copy,
            rows: told.rows.map(row => copyRow({
                name: row.text,
                side: row.side,
                copy: row.copy,
                glyph: String.fromCodePoint(row.glyph)
            })),
            note: note
        };
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
