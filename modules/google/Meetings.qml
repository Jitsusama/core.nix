// Joel's meetings in the launcher, as parts of the Calendar program
// (program:google-calendar, the desktop entry home.nix gives), from now
// to a week on, so Programs › Calendar lists them after the program's own
// acts, and a meeting's name finds it from anywhere. Each is the row the
// google daemon serves ready (src/calendar_rows.rs), worded, ordered and
// with its acts as rows inside it, held and made as every ready place's
// are (ReadyRows.qml), so the shell only draws it and words its time.
// Return joins its call from ten minutes before it starts until it ends,
// and otherwise lists its acts, its details in the drawer beside it, as
// the daemon serves the row again each minute. `next` is the next
// meeting's row, which Now shows.
//
// Calendar's Recent (recentOf, which Applications.qml asks every provider
// that has it) is the meetings Joel held lately, newest first by when
// each started, a series once at its latest, served too (`held`), each
// saying at its side when it was held. It stands, "none" while it holds
// nothing (keepsRecentOf), since the program keeps 30 days of them.
//
// Two of a meeting's acts are still the shell's, as the daemon names
// (src/calendar_inside.rs): Move the Meeting's later times and one typed
// there, which need Calendar.qml's clock and its reading of a typed time,
// made here inside the row it serves (`kind: "move"`); and Open the
// Transcript, which only call-transcripts knows, put after Show in
// Calendar.
import qs
import Quickshell
import QtQuick

Provider {
    id: provider

    what: "meeting"

    // Calendar's rows and drawers as the daemon serves them, each drawer
    // a record of its own (`drawer:calendar:<id>`). Watched whether or not
    // the launcher's open, so it opens on rows already held, and a change
    // while shut only keeps its record.
    ReadyRows {
        id: ready

        socket: provider.socket
        place: "calendar"
        beside: ["drawer"]
        // What Google held may not be Joel's to see once it answers again.
        keptThroughGap: false
        maker: provider
        drawn: provider.launcher?.open ?? false
        make: (row, key) => provider.made(ready, row, key)
    }
    // Named, so a test tells it records as the daemon would.
    readonly property alias served: ready

    // The meetings held lately, Calendar's Recent, as the daemon serves
    // them, each saying when it started (`at`), which a Recent orders by.
    ReadyRows {
        id: had

        socket: provider.socket
        place: "held"
        beside: ["drawer"]
        keptThroughGap: false
        maker: provider
        drawn: provider.launcher?.open ?? false
        make: (row, key) => provider.made(had, row, key)
    }
    readonly property alias heldServed: had

    readonly property string socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/google.sock`

    // A row as `from` serves it, made a meeting's part of Calendar, so a
    // program's Recent and Programs' counts find it, and every row inside
    // made by the meeting's component. Move the Meeting holds the times
    // Calendar.qml offers, and the meeting's own acts Open the Transcript
    // after Show in Calendar.
    function made(from, row, key) {
        const fields = { component: "meeting" };
        if (row.kind === "meeting" || row.kind === "call")
            Object.assign(fields, {
                what: "meeting",
                of: "program:google-calendar",
                part: row.name ?? "",
                today: row.today === true,
                at: row.held?.start ?? 0,
                verbs: () => provider.transcribed(from.inside(key), row),
                targets: () => provider.transcribed(from.inside(key), row)
            });
        if (row.kind === "move") {
            const meeting = Calendar.meeting(provider.meetingOf(row.parent));
            if (meeting) {
                const moving = Calendar.moving(meeting, provider);
                Object.assign(fields, {
                    verbs: moving.targets,
                    targets: moving.targets,
                    typed: moving.typed
                });
            }
        }
        return from.thing(row, key, fields);
    }

    // The id of the meeting a row inside is in, from its parent's key,
    // `row:<place>:<id>`.
    function meetingOf(parent) {
        return String(parent ?? "").split(":").slice(2).join(":");
    }

    // A meeting's acts, with Open the Transcript after Show in Calendar
    // once call-transcripts wrote its call down.
    function transcribed(inside, row) {
        const transcript = Calendar.transcribing({ id: row.held?.id ?? "" }, provider);
        if (!transcript)
            return inside;
        const seeing = inside.findIndex(one => one.key.endsWith(":see"));
        const after = seeing < 0 ? inside.length : seeing + 1;
        return inside.slice(0, after).concat([transcript], inside.slice(after));
    }

    // Its rows, made only while the launcher is open or its rows are
    // wanted (Provider.qml's wanted), as a walk of every row is, so shut
    // it gives the rows it made last and a change wakes nothing.
    readonly property bool rowsWanted: provider.launcher?.open === true || provider.wanted
    readonly property var madeRows: ({ rows: [] })
    things: {
        if (!provider.rowsWanted)
            return provider.madeRows.rows;
        const rows = ready.rows();
        provider.madeRows.rows = rows;
        return rows;
    }
    onThingsChanged: changedTo(things)

    // While the launcher is open, who's in each Meet call is asked of Meet,
    // so Now's call says it (Calendar.qml's watchingCalls); never while
    // it's closed.
    readonly property bool callsWatched: launcher !== null && launcher.open === true
    onCallsWatchedChanged: Calendar.watchingCalls += callsWatched ? 1 : -1

    // As many as a Recent lists (launcher/Recent.js), so none is made only
    // to be dropped.
    readonly property int most: 9

    readonly property var heldRows: had.rows()
    function recentOf(program) {
        return program === "google-calendar" ? heldRows.slice(0, most) : [];
    }

    function keepsRecentOf(program) {
        return program === "google-calendar";
    }

    // So Programs › Calendar, held as a meeting ends or its side moves on,
    // draws its Recent again, and only then.
    onHeldRowsChanged: changed()

    // The next meeting's row, for Now, which finds this provider by its
    // name and shows the same row, with the same acts: only while the
    // account is ready and the meeting is on, starts today or starts
    // within the hour, or else null, as on a machine without google, where
    // this provider isn't there at all.
    readonly property var next: Google.account === "ready" && Calendar.upNext(Calendar.next)
        ? things.find(row => row.key === `calendar:meeting:${Calendar.next.id}`) ?? null
        : null
}
