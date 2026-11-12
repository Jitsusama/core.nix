// A stand-in for the notices daemon (modules/notices), for a QML test of
// what the launcher makes of the notices' rows: it serves `notices`, each
// as the shell's history holds one ({ id, at, app, summary, body, read,
// live, entry }), as ready rows over the daemon interface, in the places the
// daemon serves them (row.rs): every one in `notices`, the unread saying
// so, and the unread in `notices-desktop`. Each time `notices` is said
// anew, it tells each watch only the rows that read otherwise and those
// gone, as the daemon does, so a test sees what a change remakes. A test
// waits for `told` to count up before reading what the change made. A
// search is answered with the keys of the notices whose words hold every
// word asked, in the order they're held: how the daemon ranks them is its
// own tests' (modules/notices' search.rs), and each it answers is counted
// in `searches`. A check copies it beside the test with Watch.js.
pragma ComponentBehavior: Bound
import QtQuick
import Quickshell
import Quickshell.Io
import "Watch.js" as Watch

Scope {
    id: served

    property var notices: []
    // How many changes have been told, so a test waits for its own.
    property int told: 0
    // Each watch held, by place, as { id, socket }.
    property var watches: ({})
    // Each row as last told, by key, as its JSON.
    property var said: ({})
    // The words of each search answered, in order.
    property var searches: []

    // The keys of the notices' rows a search of `words` finds.
    function found(words) {
        const asked = String(words).split(" ").filter(word => word !== "");
        return notices.filter(notice => {
            const text = `${notice.summary}\n${notice.body ?? ""}`.toLowerCase();
            return asked.every(word => text.includes(word));
        }).map(notice => `row:notices:${notice.id}`);
    }

    // A notice's row in `place`, its side saying `standing` where it's
    // given, as row.rs makes one.
    function row(notice, place, standing) {
        const body = String(notice.body ?? "");
        const words = notice.summary === "" ? body : `${notice.summary}\n${body}`;
        return {
            key: `row:${place}:${notice.id}`,
            kind: "row",
            row: {
                key: `row:${place}:${notice.id}`,
                thing: `notice:${notice.id}`,
                order: -notice.at,
                name: notice.summary,
                note: body.split("\n")[0],
                glyph: { code: "f0f3" },
                hue: "program",
                side: (standing === "" ? [] : [{ said: standing }])
                    .concat([{ at: notice.at, as: "came" }]),
                drawer: { eyebrow: notice.app, eyebrowNames: true, title: notice.summary,
                    body: body, styled: false, prose: true, picture: "", facts: [] },
                acts: [{ key: "verb:details", step: "drawer" },
                    { key: "verb:notice:read",
                        name: notice.read ? "Mark It Unread" : "Mark It Read",
                        glyph: notice.read ? "unread" : "read",
                        ask: { ask: "mark", notices: [notice.id], read: !notice.read } },
                    { key: "verb:notice:copy", copy: words }],
                held: { id: notice.id, app: notice.app, entry: notice.entry ?? "",
                    read: notice.read === true,
                    at: notice.at, live: notice.live === true, said: words }
            }
        };
    }

    // Every record of `place`, by key.
    function rowsOf(place) {
        const rows = {};
        for (const notice of notices) {
            if (place === "notices")
                rows[`row:${place}:${notice.id}`] = row(notice, place,
                    notice.read ? "" : "unread");
            else if (place === "notices-desktop" && !notice.read)
                rows[`row:${place}:${notice.id}`] = row(notice, place, "");
        }
        return rows;
    }

    // Tells each watch what reads otherwise since it was last told.
    onNoticesChanged: {
        let any = false;
        for (const place of Object.keys(watches)) {
            const now = rowsOf(place);
            const changed = Object.values(now)
                .filter(one => said[one.key] !== JSON.stringify(one));
            const gone = Object.keys(said)
                .filter(key => key.startsWith(`row:${place}:`) && !(key in now));
            for (const one of changed)
                said[one.key] = JSON.stringify(one);
            for (const key of gone)
                delete said[key];
            if (changed.length > 0 || gone.length > 0) {
                Watch.tell(watches[place], { changed: changed, gone: gone });
                any = true;
            }
        }
        if (any)
            told += 1;
    }

    SocketServer {
        active: true
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/notices.sock`
        handler: Socket {
            id: asked

            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    let answer = { id: ask.id, ok: true };
                    if (ask.ask === "hello") {
                        answer = { id: ask.id, version: 1, domain: "notices",
                            state: "running" };
                    } else if (ask.ask === "watch") {
                        const place = ask.place ?? "";
                        served.watches[place] = { socket: asked, id: ask.id };
                        const rows = Object.values(served.rowsOf(place));
                        for (const one of rows)
                            served.said[one.key] = JSON.stringify(one);
                        answer = { id: ask.id, records: rows };
                        served.told += 1;
                    } else if (ask.ask === "search") {
                        const keys = served.found(ask.words ?? "");
                        answer = { id: ask.id, keys: keys, grade: keys.length > 0 ? 0 : 3,
                            words: ask.words };
                        served.searches = served.searches.concat([ask.words]);
                    } else if (ask.ask === "unwatch") {
                        // As Desktop's watch ends with the launcher shut.
                        for (const place of Object.keys(served.watches)) {
                            if (served.watches[place].socket === asked)
                                delete served.watches[place];
                        }
                    }
                    asked.write(`${JSON.stringify(answer)}\n`);
                    asked.flush();
                }
            }
        }
    }
}
