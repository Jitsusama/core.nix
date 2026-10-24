// Draws a row's graph (Graph.qml) through Quickshell itself, in the theme's
// own face and sizes, and holds it to what design.md says of one: braille,
// ten characters whatever it shows, over a dim track of every dot, a bar
// filling from the left, a spark from the right with the newest last, a
// strip a reading for each member from the left, and the warning colour at
// 95% or when the row says it warns, never when it says it doesn't. Then
// rows (Choice.qml) at a panel's width: their graphs stand in one column
// clear of the words either side, and a figure wears no glyph while a row
// holding more wears its glyph in the edge colour. It exits 0 once every
// case holds, and otherwise 1, saying which didn't.
import Quickshell
import QtQuick

ShellRoot {
    id: root

    // Joel's panel, in logical pixels.
    readonly property int panel: 677
    readonly property string empty: "\u2800"
    readonly property string full: "\u28ff"

    FloatingWindow {
        visible: true
        implicitWidth: root.panel
        implicitHeight: 400
        color: "black"

        Column {
            width: root.panel

            Graph { id: none; reading: ({ shape: "bar", values: [0] }) }
            Graph { id: half; reading: ({ shape: "bar", values: [0.5] }) }
            Graph { id: whole; reading: ({ shape: "bar", values: [1] }) }
            Graph { id: calm; reading: ({ shape: "bar", values: [1], warn: false }) }
            Graph { id: warned; reading: ({ shape: "bar", values: [0.2], warn: true }) }
            Graph { id: spark; reading: ({ shape: "spark", values: [0, 0.25, 0.5, 0.75, 1, 0.5] }) }
            Graph { id: strip; reading: ({ shape: "strip", values: [1, 0.5, 0, 0.1] }) }
            Graph { id: nothing }

            Choice {
                id: short
                name: "Today"
                side: "40%"
                glyph: "$"
                sort: "figure"
                graph: ({ shape: "bar", values: [0.4] })
                // As long as the longest's, as the launcher counts it.
                figures: 26
            }
            Choice {
                id: long
                name: "Heavy turns in Walgit Push Policy: Protect the Main Branch"
                side: "the screen, 19200 of 19200"
                glyph: "$"
                hue: Theme.machine
                sort: "holds"
                graph: ({ shape: "spark", values: [0.1, 0.9] })
                figures: 26
            }
            Choice {
                id: act
                name: "Mute it"
                glyph: "$"
                hue: Theme.action
            }
        }
    }

    function expect(what, holds, shown) {
        if (!holds)
            throw new Error(`${what}: ${shown}`);
    }

    function codes(text) {
        return Array.from(text).map(c => c.codePointAt(0).toString(16)).join(" ");
    }

    function draws(graph, what, characters, tones) {
        expect(`${what} draws ten characters`, Array.from(graph.drawn).length === 10,
            codes(graph.drawn));
        expect(what, graph.drawn === characters, `${codes(graph.drawn)}, not ${codes(characters)}`);
        const told = graph.characters.map(character => character.tone).join(" ");
        expect(`${what}'s tones`, told === tones, told);
    }

    // Every piece of text drawn under an item, with where it's drawn, a
    // graph inside a row as one piece.
    function drawn(item, into, whole = false) {
        for (const child of item.children) {
            if (whole && child.reading !== undefined) {
                if (child.visible) {
                    const at = child.mapToItem(null, 0, 0);
                    into.push({ text: child.drawn, color: child.ink, from: at.x,
                        to: at.x + child.width });
                }
                continue;
            }
            if (child.visible && typeof child.text === "string" && child.text !== ""
                && child.contentWidth !== undefined) {
                const at = child.mapToItem(null, 0, 0);
                into.push({ text: child.text, color: child.color, from: at.x,
                    to: at.x + child.contentWidth });
            }
            drawn(child, into, whole);
        }
        return into;
    }

    function said(texts) {
        return texts.map(t => `"${t.text}" ${Math.round(t.from)} to ${Math.round(t.to)}`).join(", ");
    }

    function check() {
        try {
            const inks = n => Array(n).fill("ink").join(" ");
            const hots = n => Array(n).fill("hot").join(" ");
            draws(none, "a bar at 0", root.empty.repeat(10), inks(10));
            draws(half, "a bar at 0.5", root.full.repeat(5) + root.empty.repeat(5), inks(10));
            draws(whole, "a bar at 1", root.full.repeat(10), hots(10));
            draws(calm, "a full bar that says it doesn't warn", root.full.repeat(10), inks(10));
            draws(warned, "a bar that warns", root.full.repeat(2) + root.empty.repeat(8), hots(10));
            // The newest last: 0 and 0.25, then 0.5 and 0.75, then 1 and 0.5.
            draws(spark, "a spark", root.empty.repeat(7) + "\u2880\u28f4\u28e7",
                `${inks(7)} ink ink hot`);
            // 1 and 0.5, then nothing and a little, which still lights a dot.
            draws(strip, "a strip", "\u28e7\u2880" + root.empty.repeat(8), `hot ${inks(9)}`);
            expect("no graph draws nothing", !nothing.visible, nothing.drawn);

            // Every graph as wide as its track, and as every other.
            const widths = [none, half, whole, spark, strip].map(graph => drawn(graph, [])
                .map(t => Math.round(t.to - t.from)));
            const flat = widths.reduce((all, each) => all.concat(each), []);
            expect("every graph as wide as its track", flat.every(w => w === flat[0]) && flat[0] > 0,
                JSON.stringify(widths));

            // Rows' graphs in one column, between their words and what they
            // say at the right, each clear of the other.
            const at = choice => choice.children.find(child => child.reading !== undefined)
                .mapToItem(null, 0, 0).x;
            expect("graphs stand in one column", Math.abs(at(short) - at(long)) < 0.5,
                `${at(short)} and ${at(long)}`);
            for (const row of [short, long]) {
                const texts = drawn(row, [], true).sort((a, b) => a.from - b.from);
                const clear = texts.every((t, i) => i === 0 || texts[i - 1].to <= t.from + 0.5);
                const inside = texts.every(t => t.to <= row.width);
                expect(`${row.name}'s words clear of its graph`, clear && inside, said(texts));
            }

            // A figure wears no glyph, a row holding more its glyph in the
            // edge colour, and an act its glyph in its hue.
            const glyphOf = row => drawn(row, []).find(t => t.text === "$");
            expect("a figure wears no glyph", glyphOf(short) === undefined, said(drawn(short, [])));
            expect("a row holding more wears the edge colour",
                Qt.colorEqual(glyphOf(long).color, Theme.edge), glyphOf(long).color);
            expect("an act wears its hue", Qt.colorEqual(glyphOf(act).color, Theme.action),
                glyphOf(act)?.color);
            Qt.callLater(() => Qt.exit(0));
        } catch (error) {
            console.warn(`FAIL ${error.message}`);
            Qt.callLater(() => Qt.exit(1));
        }
    }

    Timer {
        interval: 500
        running: true
        onTriggered: root.check()
    }
}
