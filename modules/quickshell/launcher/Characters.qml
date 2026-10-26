// Characters and their codes, answered in the field: "char 65" or "ascii
// 65" shows A, "char A" or "ord A" shows 65, "0x41 as text" and "U+1F600"
// show the characters they number, and "utf8 é" shows é's bytes, each with
// its other forms as parts a step in, named first. Several characters are
// a row each, holding that character's parts. Characters.js says how. The
// names are Unicode's, built at build time into `names` (names.nix) and
// read the first time a character is asked about; without them a
// character is told by its numbers alone.
import qs
import Quickshell.Io
import QtQuick
import "Characters.js" as Characters

Provider {
    id: provider

    examples: ["char 65", "ord A", "U+1F600", "utf8 é"]

    property var table: null

    function loaded() {
        if (table === null) {
            table = Characters.prepare("");
            if (settings.names) {
                file.path = settings.names;
                table = Characters.prepare(read(file));
            }
        }
        return table;
    }

    function answer(typed) {
        // Cheap enough for every key, so the names are read only for a
        // question of characters.
        if (!Characters.asked(typed))
            return null;
        const said = Characters.answer(typed, loaded());
        if (!said)
            return null;
        return {
            headline: said.headline,
            copy: said.copy,
            rows: said.rows.map(row => made(row, "characters")),
            note: said.note
        };
    }

    // A part, named by the vocabulary and copying its value, or a
    // character of several, holding its own parts.
    function made(row, within) {
        if (row.parts) {
            const key = `${within}:${row.code}`;
            return holderRow({
                key: key,
                name: row.name,
                side: row.side,
                note: row.note,
                // A character is told by its numbers, so it wears the mark
                // a number does.
                glyph: Theme.glyph.number
            }, () => row.parts.map(part => provider.made(part, key)));
        }
        return copyRow({
            key: `${within}:${row.id}`,
            name: Words.name(`characters:${row.id}`),
            side: row.side,
            copy: row.copy,
            hue: "machine",
            glyph: ""
        });
    }

    FileView {
        id: file
        blockLoading: true
        printErrors: false
    }
}
