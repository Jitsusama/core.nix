// A graph a palette's row draws, word-sized and set in braille in the
// monospace face: always ten characters wide, whatever it shows, each
// character two readings across and four dots high. Every dot lies under it,
// dim, so its floor is the least it can show and its top the most, and a
// graph empty or full still reads as one. A bar fills from the left; a
// spark holds a history, filling from the right with the newest last; a
// strip has a reading for each member, from the left. Shade says only how
// much: the warning colour goes on a reading of 95% or more, or on all of
// it when the row says it warns. Blocks were tried first and read as too
// blocky (docs/design.md, Shape and Kinds).
//
// What it draws is { shape: "bar" | "spark" | "strip", values, warn }:
// the values between 0 and 1, one for a bar, up to twenty for the others,
// the oldest first. `warn` true wears the warning colour throughout, false
// never, as a full battery wants nothing, and left out, wherever a reading
// reaches 95%.
pragma ComponentBehavior: Bound
import QtQuick

Item {
    id: graph

    property var reading: null
    property bool selected: false

    readonly property int cells: 10
    // Two readings to a character.
    readonly property int places: 2 * cells
    readonly property int high: 4
    // How faint the dots under it are: enough to see the graph's edges,
    // too little to read as dots drawn.
    readonly property real track: 0.5
    // Where a reading wants Joel soon.
    readonly property real hot: 0.95
    // Braille's dots, bottom up, for a character's left reading and its
    // right.
    readonly property var leftDots: [0x40, 0x04, 0x02, 0x01]
    readonly property var rightDots: [0x80, 0x20, 0x10, 0x08]
    readonly property int blank: 0x2800

    readonly property color ink: selected ? Theme.text : Theme.muted

    visible: reading !== null && reading !== undefined
    implicitWidth: dots.implicitWidth
    implicitHeight: dots.implicitHeight

    // A reading kept between 0 and 1, or null for none.
    function kept(value) {
        const number = Number(value);
        return value === null || value === undefined || isNaN(number) ? null
            : Math.max(0, Math.min(1, number));
    }

    // How many of a place's dots a reading lights: any reading above
    // nothing lights one, so a little reads apart from none.
    function lit(value) {
        if (value === null || value <= 0)
            return 0;
        return Math.max(1, Math.round(value * graph.high));
    }

    function column(value, side) {
        let bits = 0;
        for (let i = 0; i < graph.lit(value); i += 1)
            bits |= side[i];
        return bits;
    }

    // Every reading in the twenty places the ten characters hold, null
    // where there's none.
    readonly property var laid: {
        const out = [];
        for (let i = 0; i < graph.places; i += 1)
            out.push(null);
        const asked = graph.reading;
        if (!asked)
            return out;
        const values = (asked.values ?? []).map(value => graph.kept(value));
        if (asked.shape === "bar") {
            const share = values[0] ?? 0;
            const filled = share <= 0 ? 0 : Math.max(1, Math.round(share * graph.places));
            for (let i = 0; i < filled; i += 1)
                out[i] = 1;
            return out;
        }
        const shown = asked.shape === "spark" ? values.slice(-graph.places)
            : values.slice(0, graph.places);
        const start = asked.shape === "spark" ? graph.places - shown.length : 0;
        shown.forEach((value, i) => {
            out[start + i] = value;
        });
        return out;
    }

    // Each character, as it's drawn and in which tone: "hot" for the
    // warning, "newest" for a spark's last, or "ink".
    readonly property var characters: {
        const asked = graph.reading;
        const out = [];
        if (!asked)
            return out;
        const bar = asked.shape === "bar";
        const whole = bar ? graph.kept((asked.values ?? [])[0]) ?? 0 : 0;
        for (let i = 0; i < graph.cells; i += 1) {
            const left = graph.laid[2 * i];
            const right = graph.laid[2 * i + 1];
            const reaches = bar ? whole >= graph.hot
                : (left ?? 0) >= graph.hot || (right ?? 0) >= graph.hot;
            const warns = asked.warn === true || (asked.warn !== false && reaches);
            const newest = asked.shape === "spark" && i === graph.cells - 1;
            out.push({
                text: String.fromCharCode(graph.blank + graph.column(left, graph.leftDots)
                    + graph.column(right, graph.rightDots)),
                tone: warns ? "hot" : newest ? "newest" : "ink"
            });
        }
        return out;
    }

    // The ten characters as drawn, whatever their tones.
    readonly property string drawn: characters.map(character => character.text).join("")

    // Those of one tone, the rest left blank so each keeps its place.
    function toned(tone) {
        return characters.map(character => character.tone === tone ? character.text
            : String.fromCharCode(graph.blank)).join("");
    }

    Text {
        id: dots
        text: String.fromCharCode(graph.blank + 0xff).repeat(graph.cells)
        color: Theme.edge
        opacity: graph.track
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
    }

    Text {
        text: graph.toned("ink")
        color: graph.ink
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
    }

    // A spark's newest reading, a step brighter, since it's how things
    // stand now.
    Text {
        text: graph.toned("newest")
        color: graph.selected ? Theme.strong : Theme.text
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
    }

    Text {
        text: graph.toned("hot")
        color: Theme.warning
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
    }
}
