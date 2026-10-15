// Draws a palette's row (Choice.qml) and its field (Prompt.qml) through
// Quickshell itself, in the theme's own face and sizes, at a panel's width,
// with words too long for them, and holds them to what design.md says of
// every surface: nothing wraps, overlaps or runs past its edge, and what
// doesn't fit ends in an ellipsis. A row's name and what it is both keep
// within the row and apart, however long either is, and a field's ghost or
// hint stops at the field's end. Words that fit are left whole. It exits 0
// once every case holds, and otherwise 1, saying which didn't.
import Quickshell
import QtQuick

ShellRoot {
    id: root

    // Joel's panel, in logical pixels.
    readonly property int panel: 677
    readonly property string long: "Heavy turns in Walgit Push Policy: Protect the Main Branch"
    readonly property string sentence: "$4,199 billed in the last 7 days against an allowance"
        + " of $15,000, which resets on Monday at midnight"

    FloatingWindow {
        visible: true
        implicitWidth: root.panel
        implicitHeight: 400
        color: "black"

        Column {
            width: root.panel

            Choice { id: bothLong; name: root.long; side: root.sentence; glyph: "$" }
            Choice { id: sideLong; name: "Weekly cap"; side: root.sentence; glyph: "$" }
            Choice { id: nameLong; name: root.long + root.long; side: "7.0× usual"; glyph: "$" }
            Choice { id: fits; name: "Today"; side: "$484 · usual"; glyph: "$" }
            // The margin's row for nothing on, as Margin.qml draws it inside
            // its 440 points: whimsy that ends in an ellipsis isn't whimsy.
            Choice {
                id: nothing
                width: 440 - 2 * Theme.inset
                quiet: true
                glyph: String.fromCodePoint(0xf0674)
                name: "Nothing's on or waiting."
            }
            Prompt {
                id: ghosted
                width: root.panel
                text: "spend"
                ghost: " = Today: $484 · usual, ends near $1,457 ($800 to $2,216), billed"
            }
            Prompt { id: hinted; width: root.panel; hint: root.sentence }
        }
    }

    // Every piece of text drawn under an item, with where it's drawn.
    function drawn(item, into) {
        for (const child of item.children) {
            if (child.visible && typeof child.text === "string" && child.text !== ""
                && child.truncated !== undefined) {
                const at = child.mapToItem(null, 0, 0);
                into.push({ text: child.text, truncated: child.truncated,
                    from: at.x, to: at.x + child.contentWidth });
            }
            drawn(child, into);
        }
        return into;
    }

    function said(texts) {
        return texts.map(t => `"${t.text}" ${Math.round(t.from)} to ${Math.round(t.to)}`)
            .join(", ");
    }

    // A row's texts keep inside it and clear of each other.
    function apart(row, edge) {
        const texts = drawn(row, []).sort((a, b) => a.from - b.from);
        const start = row.mapToItem(null, 0, 0).x;
        const inside = texts.every(t => t.from >= start && t.to <= start + edge);
        const clear = texts.every((t, i) => i === 0 || texts[i - 1].to <= t.from);
        if (!inside || !clear)
            throw new Error(`overlapping or past the edge: ${said(texts)}`);
        return texts;
    }

    function expect(what, holds, shown) {
        if (!holds)
            throw new Error(`${what}: ${shown}`);
    }

    function check() {
        try {
            const cut = texts => texts.filter(t => t.truncated).map(t => t.text);
            let texts = apart(bothLong, bothLong.width);
            expect("a long name and side both cut", cut(texts).length === 2, said(texts));
            texts = apart(sideLong, sideLong.width);
            expect("a long side cut, its name whole", cut(texts).length === 1
                && cut(texts)[0] === root.sentence, said(texts));
            texts = apart(nameLong, nameLong.width);
            expect("a long name cut, its side whole", cut(texts).length === 1
                && cut(texts)[0] === nameLong.name, said(texts));
            texts = apart(fits, fits.width);
            expect("words that fit left whole", cut(texts).length === 0, said(texts));
            texts = apart(nothing, nothing.width);
            expect("the margin's words for nothing on whole", cut(texts).length === 0,
                said(texts));
            for (const field of [ghosted, hinted]) {
                const end = field.mapToItem(null, field.end, 0).x;
                texts = drawn(field, []);
                const after = texts.find(t => t.text === (field.ghost || field.hint));
                expect("a field's ghost or hint ends at its end", after !== undefined
                    && after.truncated && after.to <= end, `${said(texts)}, end ${end}`);
            }
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
