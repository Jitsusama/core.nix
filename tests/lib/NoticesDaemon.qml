// A stand-in for the notices daemon (modules/notices), for a QML test of
// what the launcher makes of the notices' rows: it serves `notices`, each
// as the shell's history holds one ({ id, at, app, summary, body, read,
// live, entry }), as ready rows over the daemon interface, in the places the
// daemon serves them (row.rs): every one in `notices`, the unread saying
// so, the unread in `notices-desktop`, and the read in `notices-history`,
// newest done with first, each holding when, how and which source, with
// beside them how many it keeps of each source. Each time `notices` is said
// anew, it tells each watch only the rows that read otherwise and those
// gone, as the daemon does, so a test sees what a change remakes. A test
// waits for `told` to count up before reading what the change made. A
// search is answered with the keys of the notices whose words hold every
// word asked, in the order they're held: how the daemon ranks them is its
// own tests' (modules/notices' search.rs), and each it answers is counted
// in `searches`. Those of `unserved`, notices no place serves, as a page
// served leaves the rest, are found too, each with its row in the answer;
// `notices` says beside its rows how many it has rows for, those too, and
// how many each sender sent and how many of those are unread, as the
// daemon's summary does, and a `rows` ask is answered as a
// search is, every notice's key in order with the unserved rows beside
// them, each counted in `rowsAsked`; History's count holds its unserved
// too, and its `rows` ask answers every one it keeps, those of the unserved
// with their rows. A watch gets only the kinds it
// names, and is refused, as daemon-serve refuses it, when it names one
// the daemon doesn't keep, so a launcher asking for a kind the daemon
// lacks fails here rather than only against the daemon itself.
// A check copies it beside the test with Watch.js.
pragma ComponentBehavior: Bound
import QtQuick
import Quickshell
import Quickshell.Io
import "Watch.js" as Watch

Scope {
    id: served

    property var notices: []
    // The kinds the daemon keeps, and those a watch gets only by name, as
    // its domain says them (modules/notices' main.rs, DOMAIN).
    readonly property var kinds: ["notice", "row", "summary"]
    readonly property var namedOnly: ["summary"]
    // How many changes have been told, so a test waits for its own.
    property int told: 0
    // Each watch held, by place, as { id, socket, kinds }.
    property var watches: ({})
    // Each row as last told, by key, as its JSON.
    property var said: ({})
    // The words of each search answered, in order.
    property var searches: []
    // The notices a search finds that no place serves.
    property var unserved: []
    // How many `rows` asks were answered.
    property int rowsAsked: 0
    // The programs whose notices History leaves out, since For You lists
    // them as what they ask (modules/notices' notice.rs, LISTED).
    readonly property var listed: ["waiting", "pi"]

    // Which of For You's sources a notice is from, as notice.rs says it.
    function sourceOf(notice) {
        const slack = name => String(name ?? "").toLowerCase() === "slack";
        return slack(notice.app) || slack(notice.entry) ? "slack" : "desktop";
    }

    // The notices History keeps of `from`, newest done with first, of
    // `source` where it's given.
    function history(source, from = notices) {
        return from.filter(notice => notice.read && !listed.includes(notice.app))
            .filter(notice => source === undefined || sourceOf(notice) === source)
            .sort((one, other) => (other.readAt ?? other.at) - (one.readAt ?? one.at));
    }

    // The kinds a watch's `kind` asks for, as daemon-serve's Domain reads
    // them: every kind but those named only, when it names none, or
    // { unknown } naming the first the daemon doesn't keep.
    function kindsOf(named) {
        if (named === undefined || named === null)
            return kinds.filter(kind => !namedOnly.includes(kind));
        const asked = Array.isArray(named) ? named : [named];
        const unknown = asked.find(kind => !kinds.includes(kind));
        return unknown === undefined ? asked : { unknown: unknown };
    }

    // The unserved rows of `keys`, by key, bare as an answer gives them.
    function unservedOf(keys) {
        const rows = {};
        for (const notice of unserved) {
            const one = row(notice, "notices", notice.read ? "" : "unread");
            if (keys.includes(one.key))
                rows[one.key] = one.row;
        }
        return rows;
    }

    // The keys of the notices' rows a search of `words` finds.
    function found(words) {
        const asked = String(words).split(" ").filter(word => word !== "");
        return notices.concat(unserved).filter(notice => {
            const text = `${notice.summary}\n${notice.body ?? ""}`.toLowerCase();
            return asked.every(word => text.includes(word));
        }).map(notice => `row:notices:${notice.id}`);
    }

    // A notice's row in `place`, its side saying `standing` where it's
    // given, as row.rs makes one.
    function row(notice, place, standing) {
        const body = String(notice.body ?? "");
        const words = notice.summary === "" ? body : `${notice.summary}\n${body}`;
        const held = { id: notice.id, app: notice.app, entry: notice.entry ?? "",
            read: notice.read === true,
            at: notice.at, live: notice.live === true, said: words };
        // History's rows say when and how each was done with, and which
        // source it's from, muted and For You's as that source's (row.rs).
        const history = place === "notices-history";
        if (history)
            Object.assign(held, { readAt: notice.readAt ?? notice.at, how: "read",
                source: sourceOf(notice) });
        const forYou = history ? { forYou: { kind: "notice", source: sourceOf(notice) } } : {};
        return {
            key: `row:${place}:${notice.id}`,
            kind: "row",
            row: Object.assign(forYou, {
                key: `row:${place}:${notice.id}`,
                thing: `notice:${notice.id}`,
                order: -notice.at,
                name: notice.summary,
                note: body.split("\n")[0],
                glyph: { code: "f0f3" },
                hue: history ? "muted" : "program",
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
                held: held
            })
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
        if (place === "notices-history") {
            for (const notice of history())
                rows[`row:${place}:${notice.id}`] = row(notice, place, "");
            // Counted with those past the page, as `unserved` stands for.
            const every = notices.concat(unserved);
            rows["summary:notices-history"] = { key: "summary:notices-history",
                kind: "summary", summary: { count: history(undefined, every).length, sources: {
                    desktop: history("desktop", every).length,
                    slack: history("slack", every).length } } };
        }
        if (place === "notices") {
            const every = notices.concat(unserved);
            const senders = {};
            for (const one of every) {
                const sent = senders[one.app ?? ""] ?? (senders[one.app ?? ""] = { count: 0,
                    unread: 0 });
                sent.count += 1;
                sent.unread += one.read ? 0 : 1;
            }
            rows["summary:notices"] = { key: "summary:notices", kind: "summary",
                summary: { count: every.length, senders: senders } };
        }
        return rows;
    }

    // Tells each watch what reads otherwise since it was last told.
    onNoticesChanged: {
        let any = false;
        for (const place of Object.keys(watches)) {
            const now = rowsOf(place);
            const changed = Object.values(now)
                .filter(one => watches[place].kinds.includes(one.kind))
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
                    } else if (ask.ask === "watch"
                        && served.kindsOf(ask.kind).unknown !== undefined) {
                        const kind = served.kindsOf(ask.kind).unknown;
                        answer = { id: ask.id, error: { code: "unknown-kind",
                            message: `no kind ${kind} here` } };
                    } else if (ask.ask === "watch") {
                        const place = ask.place ?? "";
                        const kinds = served.kindsOf(ask.kind);
                        served.watches[place] = { socket: asked, id: ask.id, kinds: kinds };
                        const rows = Object.values(served.rowsOf(place))
                            .filter(one => kinds.includes(one.kind));
                        for (const one of rows)
                            served.said[one.key] = JSON.stringify(one);
                        answer = { id: ask.id, records: rows };
                        served.told += 1;
                    } else if (ask.ask === "search") {
                        const keys = served.found(ask.words ?? "");
                        answer = { id: ask.id, keys: keys, rows: served.unservedOf(keys),
                            grade: keys.length > 0 ? 0 : 3, words: ask.words };
                        served.searches = served.searches.concat([ask.words]);
                    } else if (ask.ask === "rows" && ask.place === "notices-history") {
                        // Every one History keeps, with the rows of those
                        // past the page beside.
                        const place = "notices-history";
                        const keys = served.history(ask.source,
                            served.notices.concat(served.unserved))
                            .map(notice => `row:${place}:${notice.id}`);
                        const rows = {};
                        for (const notice of served.history(ask.source, served.unserved))
                            rows[`row:${place}:${notice.id}`] = served.row(notice, place, "").row;
                        answer = { id: ask.id, keys: keys, rows: rows };
                        served.rowsAsked += 1;
                    } else if (ask.ask === "rows") {
                        const keys = served.notices.concat(served.unserved)
                            .sort((one, other) => other.at - one.at)
                            .map(notice => `row:notices:${notice.id}`);
                        answer = { id: ask.id, keys: keys, rows: served.unservedOf(keys) };
                        served.rowsAsked += 1;
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
