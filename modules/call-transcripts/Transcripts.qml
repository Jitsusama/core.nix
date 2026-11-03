pragma Singleton
pragma ComponentBehavior: Bound
// Every call call-transcripts has written down, as the shell holds them:
// read from its daemon on $XDG_RUNTIME_DIR/daemons/calls.sock through
// Daemon.qml, one connection watching every call, never from the Calls
// folder. A machine without call-transcripts has no socket, which
// Daemon.qml waits on silently, so nothing shows.
//
// `row` is a call's one row, which every surface showing one takes as it
// is, as Calendar.row is a meeting's: the launcher's Transcripts
// (Transcribed.qml beside this), someone's Recent in People
// (TranscriptSource.qml) and a meeting's Open the Transcript all show
// the same thing, with the same acts and details. Transcripts.js works
// out what it says.
import qs
import Quickshell
import QtQuick
import "Transcripts.js" as Said

Singleton {
    id: root

    Daemon {
        id: daemon

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/calls.sock`
        kind: "call"
    }

    // Every call, newest first, made again only as the daemon says one
    // changed.
    readonly property var calls: daemon.revision >= 0 ? Said.newestFirst(Object.values(daemon.held))
        : []

    // What a row says beside its name and below it, and its drawer's
    // facts' names, the vocabulary's.
    readonly property var said: Object.assign({}, Words.of("transcript").said ?? {}, {
        length: Words.name("transcript:length"),
        with: Words.name("transcript:with")
    })

    // The newest call of a meeting, by its calendar event, or null.
    function ofEvent(event) {
        return Said.ofEvent(calls, event);
    }

    // The calls with someone, by any of their addresses, newest first.
    function withThem(emails) {
        return calls.filter(call => Said.withThem(call, emails));
    }

    // Its name: the call's title, a meeting's or who a Tuple call was
    // with; for a call that had neither, who was on it, "Call With Sara
    // Park", or failing that when it began, "Call at 14:00", since Joel
    // remembers a call by who was on it.
    function titled(call, who) {
        if (call.title !== "")
            return call.title;
        const on = who || call.person;
        return on ? Words.name("transcript", "untitled", { who: on })
            : Words.name("transcript", "unknown", { time: Said.clock(call.started) });
    }

    // Who it was with, as People knows them, or "" when it doesn't, or
    // there's no People: any provider holding `personOf` (People.qml).
    function personName(call, provider) {
        if (!call.person)
            return "";
        const people = (provider.launcher?.sources ?? []).find(source => typeof source.personOf
            === "function");
        return people?.personOf(call.person)?.name ?? "";
    }

    // A call's row, made with the provider of the surface showing it, whose
    // `thing` and launcher it uses: its title, what it was at its right,
    // when it was and how long below, or that it's being written down,
    // and its summary or how it opens in the drawer. Return opens its
    // transcript in the reader, closing the launcher; with none written
    // yet, Return steps in to what can be done. Its other words are who
    // it was with, so their name finds it as their address does.
    function row(call, provider) {
        const who = root.personName(call, provider);
        const name = root.titled(call, who);
        const held = root.fields(call, name, who, () => root.acts(call, provider));
        const opens = typeof call.transcript === "string" && call.transcript !== "";
        const made = opens ? root.opening(call, provider, held) : root.entering(provider, held);
        // Its details, as they stand when they're shown, so who it was
        // with is named once People knows them, however long ago the row
        // was made.
        Object.defineProperty(made, "drawer", {
            enumerable: true,
            get: () => Said.details(call, name, root.said, root.personName(call, provider))
        });
        return made;
    }

    // What every call's row holds, whatever Return does.
    function fields(call, name, who, acts) {
        return {
            key: `transcript:${call.key}`,
            what: "transcript",
            component: "transcript",
            name: name,
            glyph: Words.glyph("transcript"),
            hue: "program",
            side: root.said[Said.kind(call)] ?? "",
            note: root.noted(call, Date.now()),
            keywords: [who, call.person ?? ""].filter(word => word !== "").join(" ") || null,
            at: call.started,
            call: call,
            verbs: acts
        };
    }

    // A call with a transcript: Return opens it.
    function opening(call, provider, held) {
        return provider.thing(Object.assign(held, {
            verb: Words.verb("verb:transcript:open"),
            later: true,
            run: () => root.open(call.transcript)
        }));
    }

    // A call with nothing written yet: Return steps in to what can be done.
    function entering(provider, held) {
        const row = provider.thing(Object.assign(held, {
            verb: "more",
            stays: true,
            targets: held.verbs,
            run: () => provider.launcher?.enter(row)
        }));
        return row;
    }

    // When it was and how long, as of `now`, as its note says it.
    function noted(call, now) {
        return Said.note(call, now, root.said);
    }

    // Its acts, in the one order every surface lists them: Open the
    // Transcript, once there's one, Open Its Folder, then, last as copying
    // always is, Copy the Path, the transcript's, or its folder's while
    // there's none.
    function acts(call, provider) {
        const act = (id, fields) => (fields.copy ? provider.copyRow : provider.thing)(Object.assign({
            key: `verb:transcript:${id}`,
            name: Words.name(`verb:transcript:${id}`),
            verb: Words.verb(`verb:transcript:${id}`),
            glyph: Words.glyph(`verb:transcript:${id}`),
            hue: "action"
        }, fields));
        const path = call.transcript || call.folder;
        const rows = [];
        if (call.transcript)
            rows.push(act("open", { later: true, run: () => root.open(call.transcript) }));
        rows.push(act("folder", { later: true, run: () => root.open(call.folder) }));
        rows.push(act("copy", { copy: path }));
        return rows;
    }

    // Opens a file or folder as any is opened, a transcript in the
    // session's reader.
    function open(path) {
        Niri.spawn(["xdg-open", path]);
    }
}
