// A stand-in for the to-do daemon (modules/to-do), for a QML test of what
// the launcher makes of the looks done with: it serves `looks`, each as
// { id, kind, title, why, how, doneAt } and who left it where a case says
// (session, address, quest, cell), as ready rows in the place
// the daemon serves History's looks, `looks-history`, newest done with
// first, each holding the look as the daemon read it, which names no file,
// with beside them how many there are (modules/to-do's history.rs). Each time `looks` is said anew it tells each watch only
// the rows that read otherwise and those gone, as the daemon does, and
// counts it in `told`. It keeps only the kinds a History watch asks for,
// so the shell's watch of every look file whole is refused, as
// daemon-serve refuses a kind it lacks, and a case's own look texts stand.
// A check copies it beside the test with Watch.js.
pragma ComponentBehavior: Bound
import QtQuick
import Quickshell
import Quickshell.Io
import "Watch.js" as Watch

Scope {
    id: served

    property var looks: []
    readonly property var kinds: ["row", "summary"]
    property int told: 0
    // Each watch held, by place, as { id, socket, kinds }.
    property var watches: ({})
    // Each row as last told, by key, as its JSON.
    property var said: ({})

    // A look's row as the daemon's looks-history serves it.
    function row(look, at) {
        const key = `row:looks-history:${look.id}`;
        return {
            key: key,
            kind: "row",
            row: {
                key: key,
                thing: `look:${look.id}`,
                place: "looks-history",
                kind: "look",
                order: at * 1024,
                name: look.title,
                note: look.why,
                glyph: { code: "f06e" },
                hue: "muted",
                side: [{ said: `${look.how} {time}`, at: look.doneAt, as: "moment" }],
                drawer: { title: look.title, facts: [{ name: "Resolved",
                    said: `${look.how} {time}`, at: look.doneAt, as: "moment" }],
                    nerd: [{ label: "Look", value: look.id }], body: look.why, prose: true },
                acts: look.kind === "terminal" ? []
                    : [{ key: "verb:on:look:open", open: "https://example.com" }],
                held: { id: look.id, kind: look.kind, title: look.title,
                    target: "https://example.com", state: look.how, how: look.how,
                    session: look.session ?? null, address: look.address ?? null,
                    quest: look.quest ?? null, cell: look.cell ?? null,
                    at: look.doneAt, resolvedAt: look.doneAt, doneAt: look.doneAt },
                forYou: { kind: "look", source: "looks" }
            }
        };
    }

    // Every record of `place`, by key.
    function rowsOf(place) {
        const rows = {};
        if (place !== "looks-history")
            return rows;
        looks.slice().sort((one, other) => other.doneAt - one.doneAt)
            .forEach((look, at) => {
                rows[`row:${place}:${look.id}`] = row(look, at);
            });
        rows[`summary:${place}`] = { key: `summary:${place}`, kind: "summary",
            count: looks.length };
        return rows;
    }

    onLooksChanged: {
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
        path: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/to-do.sock`
        handler: Socket {
            id: asked

            parser: SplitParser {
                onRead: line => {
                    const ask = JSON.parse(line);
                    const named = Array.isArray(ask.kind) ? ask.kind : [ask.kind ?? "row"];
                    const unknown = named.find(kind => !served.kinds.includes(kind));
                    let answer = { id: ask.id, ok: true };
                    if (ask.ask === "hello") {
                        answer = { id: ask.id, version: 1, domain: "to-do", state: "running" };
                    } else if (ask.ask === "watch" && unknown !== undefined) {
                        answer = { id: ask.id, error: { code: "unknown-kind",
                            message: `no kind ${unknown} here` } };
                    } else if (ask.ask === "watch") {
                        const place = ask.place ?? "";
                        served.watches[place] = { socket: asked, id: ask.id, kinds: named };
                        const rows = Object.values(served.rowsOf(place))
                            .filter(one => named.includes(one.kind));
                        for (const one of rows)
                            served.said[one.key] = JSON.stringify(one);
                        answer = { id: ask.id, records: rows };
                        served.told += 1;
                    } else if (ask.ask === "unwatch") {
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
