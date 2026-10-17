// The clipboard's history, a collection: every text copy made this session,
// newest first, as clipboard-history hears them (clipboard/src/main.rs).
// One connection to its socket brings every copy kept, then each new one,
// so nothing is started per copy and nothing is written to disk. A search
// with a match adds one row; the right arrow, or Super+V, shows them all.
// Return puts the copy back on the clipboard. Forget the Clipboard, found
// by its words while there's something to forget, has the helper forget
// every copy, and what the clipboard holds with them.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    collection: ({
        noun: "copies",
        title: "Clipboard",
        glyph: String.fromCodePoint(0xf0a38),
        ask: "which copy?"
    })

    // Newest first.
    property var all: []

    readonly property var forget: thing({
        key: "copies:forget",
        verb: "forget",
        name: "Forget the Clipboard",
        keywords: "clipboard copies history clear erase wipe",
        glyph: String.fromCodePoint(0xf01fe),
        hue: "action",
        run: () => {
            if (link.item !== null) {
                link.item.write("clear\n");
                link.item.flush();
            }
        }
    })

    things: all.length > 0 ? [forget] : []

    readonly property string socket:
        `${Quickshell.env("XDG_RUNTIME_DIR")}/clipboard-history.sock`

    // How a copy reads in a row: its lines run together, cut short.
    function shown(text) {
        const flat = text.replace(/\s+/g, " ").trim();
        return flat.length > 120 ? `${flat.slice(0, 119)}…` : flat;
    }

    // How long ago, in the fewest words.
    function ago(at) {
        const seconds = Math.max(0, (Date.now() - at) / 1000);
        if (seconds < 60)
            return "just now";
        if (seconds < 3600)
            return `${Math.floor(seconds / 60)} min ago`;
        if (seconds < 86400)
            return `${Math.floor(seconds / 3600)} h ago`;
        return `${Math.floor(seconds / 86400)} d ago`;
    }

    // What a copy is, by its glyph, so a link, a path or a number stands
    // out from the text around it.
    function kind(text, lines) {
        const one = text.trim();
        if (lines > 1)
            return 0xf09aa;
        if (/^[a-z][a-z0-9+.-]*:\/\/\S+$/i.test(one))
            return 0xf0337;
        if (/^[^\s@]+@[^\s@]+\.[a-z]{2,}$/i.test(one))
            return 0xf01f0;
        if (/^(~|\.{0,2})\/\S*$/.test(one))
            return 0xf0224;
        if (/^[-+]?[\d,_ ]*\.?\d+$/.test(one))
            return 0xf03a0;
        return 0xf0a38;
    }

    function made(copy) {
        const lines = copy.text.trim().split("\n").length;
        return member({
            key: `copy:${shown(copy.text)}`,
            verb: "copy",
            name: shown(copy.text),
            glyph: String.fromCodePoint(kind(copy.text, lines)),
            hue: "text",
            at: copy.at,
            text: copy.text,
            lines: lines,
            // Copied while the launcher holds the keys, which the clipboard
            // needs.
            now: true,
            run: () => {
                Quickshell.clipboardText = copy.text;
            }
        });
    }

    // Each row's age, as of now, and how many lines it has.
    function aged(rows) {
        for (const row of rows)
            row.side = row.lines > 1 ? `${row.lines} lines · ${ago(row.at)}` : ago(row.at);
        return rows;
    }

    function heard(line) {
        let said;
        try {
            said = JSON.parse(line);
        } catch (e) {
            return;
        }
        if (said.clear) {
            all = [];
            Report.say("forgot the copies");
            return;
        }
        const kept = all.filter(row => row.text !== said.text);
        kept.unshift(made(said));
        all = kept.slice(0, settings.keep ?? 200);
        Report.say(`heard copy ${said.text.split("\n")[0]}`);
    }

    function search(typed) {
        const words = Fuzzy.words(typed);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const row of all) {
            if ((row.prepared.mask & need) !== need)
                continue;
            const m = Fuzzy.match(row.prepared, words, false);
            if (m.score >= 0)
                found.push({ item: row.prepared, score: m.score, grade: m.grade, row: row });
        }
        // Newest first among equals, not shortest, since a copy is wanted
        // by when it was made.
        let finest = 3;
        for (const f of found)
            finest = Math.min(finest, f.grade);
        const kept = found.filter(f => f.grade === finest)
            .sort((a, b) => b.score - a.score || b.row.at - a.row.at);
        return { rows: aged(kept.map(f => f.row)), grade: finest };
    }

    function everything() {
        return aged(all.slice());
    }

    // A fresh connection each time, since one refused or dropped doesn't
    // try again by itself. What it sends again replaces what was held.
    LazyLoader {
        id: link
        active: true

        Socket {
            path: provider.socket
            connected: true
            parser: SplitParser {
                onRead: line => provider.heard(line)
            }
            onConnectionStateChanged: {
                if (connected)
                    provider.all = [];
                else
                    again.restart();
            }
            onError: again.restart()
        }
    }

    Timer {
        id: again
        interval: 2000
        onTriggered: {
            link.active = false;
            link.active = true;
        }
    }
}
