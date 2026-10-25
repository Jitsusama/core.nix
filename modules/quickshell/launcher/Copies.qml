// The clipboard's history, a collection: every copy made this session, text
// and pictures, newest first, as clipboard-history hears them
// (clipboard/src/main.rs). One connection to its socket brings every copy
// kept, then each new one, so nothing is started per copy and nothing is
// written to disk. A search with a match adds one row; the right arrow, or
// Super+V, shows them all. A text copy's row runs its lines together, cut
// short, and the selected one reads whole below the rows. A picture's row
// is named by its size, and the launcher shows the picture itself while
// it's selected (`picture`, its file). Return pastes the copy into the
// window the launcher opened over: it goes back on the clipboard, a
// picture by asking the helper, which holds it, and once the window has
// the keys again, its paste keys are pressed for it (Pasting.qml, which
// Super+E's emoji paste through too). With no window there,
// Return only copies, as the right arrow's Copy always does. Forget the
// Clipboard, found by its words while there's something to forget, has the
// helper forget every copy, and what the clipboard holds with them.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "../Fuzzy.js" as Fuzzy

Provider {
    id: provider

    what: "copy"

    collection: Words.collection("clipboard")

    // Newest first.
    property var all: []

    readonly property var forget: thing({
        key: "copies:forget",
        // An act, not a copy, so it says no kind (docs/design.md).
        what: "",
        verb: Words.verb("copies:forget"),
        name: Words.name("copies:forget"),
        keywords: `clipboard copies history ${Words.keywords("copies:forget")}`,
        glyph: Words.glyph("copies:forget"),
        hue: "action",
        run: () => provider.tell("clear")
    })

    // Asks the helper for something, a line at a time.
    function tell(line) {
        const socket = link.item as Socket;
        if (socket !== null) {
            socket.write(`${line}\n`);
            socket.flush();
        }
    }

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

    // What a copy is, by the theme's mark for it, so a link, a path or a
    // number stands out from the text around it.
    function kind(text, lines) {
        const one = text.trim();
        if (lines > 1)
            return Theme.glyph.lines;
        if (/^[a-z][a-z0-9+.-]*:\/\/\S+$/i.test(one))
            return Theme.glyph.link;
        if (/^[^\s@]+@[^\s@]+\.[a-z]{2,}$/i.test(one))
            return Theme.glyph.email;
        if (/^(~|\.{0,2})\/\S*$/.test(one))
            return Theme.glyph.path;
        if (/^[-+]?[\d,_ ]*\.?\d+$/.test(one))
            return Theme.glyph.number;
        return Theme.glyph.clipboard;
    }

    function made(copy) {
        const lines = copy.text.trim().split("\n").length;
        const row = member({
            key: `copy:${shown(copy.text)}`,
            name: shown(copy.text),
            glyph: kind(copy.text, lines),
            hue: "text",
            at: copy.at,
            text: copy.text,
            // As it was copied, lines and all, which the row cuts short.
            reads: copy.text.trim(),
            about: lines > 1 ? `${lines} lines` : "",
            // Copied while the launcher holds the keys, which the clipboard
            // needs.
            now: true
        });
        return pastes(row, () => {
            Quickshell.clipboardText = copy.text;
        });
    }

    // A row that pastes on Return, and steps into Paste and Copy, or Copy
    // alone with no window to paste into. Its verb follows, so the foot says
    // which Return does.
    function pastes(row, copied) {
        row.run = () => Pasting.paste(copied);
        Object.defineProperty(row, "verb", {
            enumerable: true,
            get: () => Pasting.verb()
        });
        row.verbs = () => {
            const act = (id, run) => thing({
                key: id,
                verb: Words.verb(id),
                name: Words.name(id),
                keywords: Words.keywords(id),
                glyph: Words.glyph(id),
                hue: "action",
                now: true,
                run: run
            });
            const copy = act("verb:copy:copy", copied);
            return Pasting.under() === null ? [copy]
                : [act("verb:copy:paste", () => Pasting.paste(copied)), copy];
        };
        return row;
    }

    // A picture's type and size, in the fewest words.
    readonly property var kinds: ({
        "image/png": "PNG",
        "image/jpeg": "JPEG",
        "image/webp": "WebP",
        "image/gif": "GIF"
    })

    function weighed(bytes) {
        if (bytes < 1024)
            return `${bytes} B`;
        if (bytes < 1024 * 1024)
            return `${Math.round(bytes / 1024)} KB`;
        return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    }

    // A picture's row, named by its width and height when its type says
    // them, which a search for "image" finds.
    function pictured(copy) {
        const size = copy.width ? ` ${copy.width} \u00d7 ${copy.height}` : "";
        const row = member({
            key: `picture:${copy.picture}`,
            name: `Image${size}`,
            glyph: Theme.glyph.picture,
            hue: "text",
            at: copy.at,
            id: copy.picture,
            picture: copy.path,
            across: copy.width ?? 0,
            down: copy.height ?? 0,
            about: `${kinds[copy.type] ?? "Picture"} \u00b7 ${weighed(copy.bytes)}`
        });
        return pastes(row, () => provider.tell(`copy ${copy.picture}`));
    }

    // Each row's age, as of now, after what it is.
    function aged(rows) {
        for (const row of rows)
            row.side = row.about !== "" ? `${row.about} \u00b7 ${ago(row.at)}` : ago(row.at);
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
        // Pictures the helper let go to make room, whose files are gone.
        if (said.gone) {
            all = all.filter(row => row.picture === undefined || !said.gone.includes(row.id));
            Report.say(`let go ${said.gone.length === 1 ? "a picture" : `${said.gone.length} pictures`}`);
            return;
        }
        const picture = said.picture !== undefined;
        const kept = all.filter(row => picture ? row.id !== said.picture : row.text !== said.text);
        const row = picture ? pictured(said) : made(said);
        kept.unshift(row);
        all = kept.slice(0, settings.keep ?? 200);
        Report.say(`heard copy ${picture ? row.name : said.text.split("\n")[0]}`);
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
