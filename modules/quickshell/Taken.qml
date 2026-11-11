pragma Singleton
// What was taken lately, the one reader of the folders captures land in:
// screenshots, recordings and the camera's, for Capture's Recent
// (launcher/Capture.qml) and the image programs' own screenshot rows
// (modules/gimp, modules/swayimg). Each names the folders it shows
// (`want`), which must be among those the programs daemon reads for it,
// `listed:shared:taken` (modules/quickshell/home.nix). It reads them
// itself, with no command, so the shell starts no process for them (Joel,
// 2026-11-10: the daemons are the data layer, reached over a socket held
// open). They're watched from the shell's start until the launcher first
// rests, and again from each opening (`read`, which Capture asks) until it
// closes (`rest`), each watch beginning reading them afresh if one changed
// since, so nothing is read at rest (the shell's design,
// RSCH-20261029-0E8050). The files are still made here from the daemon's
// lines, a stopgap until it serves Capture's and the image programs' rows
// as ready rows, with Programs' own by Thu 12 Nov.
//
// It finds what it read by name (`named`), each name made ready to match
// once, and screenshots by the words in them (`search`), asking the
// screen-text daemon (modules/screen-text) over one connection of its own
// through Daemon.qml. That's made only once a search is asked, and let go
// when what searched rests (`rest`), as the launcher closing does
// (ScreenText.qml), so no socket is held at rest. It watches no record,
// since a search's answer is all it needs: each text is asked once, and
// the last few answers are kept through a rest, so typing back over a word
// asks nothing again.
import Quickshell
import QtQuick
import "Fuzzy.js" as Fuzzy

Singleton {
    id: root

    // The folders read, each named by what shows it.
    property var folders: []
    // Every file in them, newest first: { path, folder, at, size }, size in
    // bytes, for Capture's drawer, and at in
    // milliseconds since the epoch.
    property var files: []
    // The programs daemon's key for what's in the folders.
    readonly property string key: "listed:shared:taken"
    // Whether they're watched now: from the first want until the launcher
    // rests, and from each opening.
    property bool wanted: false

    function want(folder) {
        if (root.folders.includes(folder))
            return;
        root.folders = root.folders.concat([folder]);
        root.read();
    }

    function read() {
        if (root.folders.length > 0)
            root.wanted = true;
    }

    // The newest file in a folder whose name passes, or none.
    function newestIn(folder, named) {
        return root.files.find(file => file.folder === folder && named.test(file.path)) ?? null;
    }

    function countIn(folder, named) {
        return root.files.filter(file => file.folder === folder && named.test(file.path)).length;
    }

    // The daemon's lines, as find prints them: a file's time in seconds, its
    // size and its path each.
    function heard(lines) {
        root.files = lines.filter(line => line.includes("\t"))
            .map(line => {
                const first = line.indexOf("\t");
                const second = line.indexOf("\t", first + 1);
                const path = line.slice(second + 1);
                return {
                    path: path,
                    folder: path.slice(0, path.lastIndexOf("/")),
                    at: Number(line.slice(0, first)) * 1000,
                    size: Number(line.slice(first + 1, second))
                };
            })
            .sort((a, b) => b.at - a.at);
    }

    // The paths in these folders whose names what's typed finds, newest
    // first, as Recent lists them, the newest `most` of them: a short word
    // such as a year finds nearly every file, and more rows than a screen
    // holds are never seen. So it stops at the `most`th, and a key costs at
    // most that many matches, however many files there are: ranking them
    // by how well each matched instead had to match every one, 18 ms a key
    // over 3,000 screenshots' names. Every name a word typed starts a word
    // of is found, as anywhere in the launcher. The properties are read
    // once rather than on every file, since each read crosses into the
    // engine's objects, and a name without the letters typed is passed over
    // before it's matched, as Fuzzy.js asks of a caller over many.
    function named(typed, within, most) {
        const words = Fuzzy.words(typed);
        if (words.length === 0 || most <= 0)
            return [];
        const files = root.files;
        const prepared = root.prepared;
        const among = new Set(within);
        const need = Fuzzy.letters(words);
        const found = [];
        for (const file of files) {
            if (!among.has(file.folder))
                continue;
            let item = prepared[file.path];
            if (item === undefined)
                item = prepared[file.path] = Fuzzy.prepare(file.path.slice(file.folder.length + 1));
            if ((item.mask & need) !== need)
                continue;
            if (Fuzzy.match(item, words, false).score < 0)
                continue;
            found.push(file.path);
            if (found.length === most)
                break;
        }
        return found;
    }

    // Each file's name made ready to match, by its path, once; a file gone
    // takes its own with it when the folders are read again.
    property var prepared: ({})

    onFilesChanged: {
        const kept = {};
        for (const file of root.files)
            if (file.path in root.prepared)
                kept[file.path] = root.prepared[file.path];
        root.prepared = kept;
    }

    // What searched rests, as the launcher does once it closes: the
    // connection is let go, and a text still asked is asked again next
    // time, while the answers kept stay.
    function rest() {
        root.wanted = false;
        reader.active = false;
        root.pending = [];
        root.searches = root.searches.filter(search => search.records !== null);
    }

    // The last few searches, newest last: { words, records }, records null
    // while it's asked, each record a screenshot's path and the line that
    // matched.
    property var searches: []
    readonly property int keeps: 4
    // The fewest letters asked about: typed a key at a time, every key
    // would otherwise ask a text of its own, and a letter or two is in
    // nearly every screenshot's words.
    readonly property int fewest: 3
    // The longest a matching line is given, in characters: it's a row's
    // note, which reads as a sentence rather than a screenful.
    readonly property int noteMost: 80

    // Said once a search's answer comes, for what showed it to look again.
    signal searched(string words)

    // The screenshots whose words hold every one of these, best first, as
    // [{ path, line }], or null while they're asked; asked at most once a
    // text, on the one connection. Fewer letters than `fewest` find none.
    function search(words) {
        const text = root.searchedAs(words);
        if (text.replace(/ /g, "").length < root.fewest)
            return [];
        const kept = root.searches.find(search => search.words === text);
        if (kept)
            return kept.records;
        root.searches = root.searches.concat([{ words: text, records: null }])
            .slice(-root.keeps);
        root.pending = root.pending.concat([text]).slice(-root.keeps);
        reader.active = true;
        root.ask();
        return null;
    }

    // Words as they're asked, and as `searched` names them: in lower case,
    // their spaces single.
    function searchedAs(words) {
        return String(words).trim().toLowerCase().split(/\s+/).join(" ");
    }

    // Texts waiting for the connection, asked once it's made, the last few.
    property var pending: []

    function ask() {
        const daemon = reader.item;
        if (!daemon || daemon.wire === null)
            return;
        const asking = root.pending;
        root.pending = [];
        for (const text of asking)
            daemon.find({ ask: "search", kind: "screenshot", words: text }, answer => {
                root.answered(text, answer);
            });
    }

    // A line as read, given as a note: one line, its spaces single, cut at
    // word edges to fit within `noteMost`, with nothing added to say it was
    // cut. It begins at the start when the first word of the text matched
    // falls within that, and otherwise two words before it, so the note
    // always says why the screenshot was found.
    function noted(line, text) {
        const whole = String(line ?? "").trim().split(/\s+/).join(" ");
        if (whole.length <= root.noteMost)
            return whole;
        const lower = whole.toLowerCase();
        const found = text.split(" ").map(word => ({ at: lower.indexOf(word), word: word }))
            .filter(match => match.at >= 0).sort((a, b) => a.at - b.at)[0];
        let from = 0;
        if (found && found.at + found.word.length > root.noteMost) {
            // Where each word begins, up to the one matched.
            const starts = [0];
            let space = whole.indexOf(" ");
            while (space >= 0 && space < found.at) {
                starts.push(space + 1);
                space = whole.indexOf(" ", space + 1);
            }
            from = starts[Math.max(0, starts.length - 3)];
        }
        const rest = whole.slice(from);
        if (rest.length <= root.noteMost)
            return rest;
        const cut = rest.lastIndexOf(" ", root.noteMost);
        return rest.slice(0, cut > 0 ? cut : root.noteMost);
    }

    // A text's answer kept, unless the daemon wasn't there to give one,
    // when it's let go to be asked again.
    function answered(text, answer) {
        if (answer === null || answer.error) {
            root.searches = root.searches.filter(search => search.words !== text);
            return;
        }
        const records = (answer.records ?? []).filter(record => typeof record.path === "string")
            .map(record => ({ path: record.path, line: root.noted(record.line, text) }));
        root.searches = root.searches.map(search => search.words !== text ? search
            : { words: text, records: records });
        root.searched(text);
    }

    LazyLoader {
        id: reader

        active: false

        Daemon {
            socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/screen-text.sock`
            kind: "screenshot"
            // No record is watched: a search's answer is given apart.
            watching: ({ keys: [] })
            onWireChanged: root.ask()
        }
    }

    // What's in the folders, as the programs daemon reads them.
    Daemon {
        id: programs

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/programs.sock`
        kind: "listed"
        live: false
        ahead: root.wanted ? [root.key] : []

        onHeard: changed => {
            if (changed.includes(root.key))
                root.heard(programs.held[root.key]?.lines ?? []);
        }
    }
}
