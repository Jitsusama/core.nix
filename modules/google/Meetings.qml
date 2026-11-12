// Joel's meetings in the launcher, as parts of the Calendar program
// (program:google-calendar, the desktop entry home.nix gives), from now
// to a week on, so Programs › Calendar lists them after the program's own
// acts, and a meeting's name finds it from anywhere. Each is the row the
// google daemon serves ready (src/calendar_rows.rs), worded, ordered and
// and with its drawer, held and made as every ready place's are
// (ReadyRows.qml), so the shell only draws it and words its time, the
// daemon serving it again each minute. `next` is the next meeting's row,
// which Now shows.
//
// Calendar's Recent (recentOf, which Applications.qml asks every provider
// that has it) is the meetings Joel held lately, newest first by when
// each started, a series once at its latest, served too (`held`), each
// saying at its side when it was held. It stands, "none" while it holds
// nothing (keepsRecentOf), since the program keeps 30 days of them.
//
// A meeting's Return and what's inside it are still Calendar.qml's
// (Calendar.row's, as Now and Meet show it): Return joins its call from
// ten minutes before it starts, and otherwise steps in to Show in
// Calendar, and its acts are the shell's, since the rows the daemon serves
// inside it (src/calendar_inside.rs) don't yet do all Calendar.qml's do:
// its guests copy an address rather than stepping through to the person,
// a document on it isn't the document's own row, and Move the Meeting and
// Open the Transcript are the shell's alone. The google daemon takes those
// next, guests and documents first, and then the meeting's acts are its.
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

    // A meeting's row as `from` serves it, made a meeting's part of
    // Calendar, so a program's Recent and Programs' counts find it, with
    // Return and its acts Calendar.qml's, as it holds the meeting now.
    // Like Calendar.row's, it holds verbs and no targets, so a step into
    // one that stays (Show in Calendar) is led by Return's own act
    // (Launcher.qml's keepsOpen), where the served inside rows would be.
    function made(from, row, key) {
        const id = row.held?.id ?? "";
        const meeting = Calendar.meeting(id) ?? Calendar.had.find(each => each.id === id) ?? null;
        const fields = {
            what: "meeting",
            component: "meeting",
            of: "program:google-calendar",
            part: row.name ?? "",
            today: row.today === true,
            at: row.held?.start ?? 0
        };
        if (meeting) {
            const joins = Calendar.joinable(meeting);
            const acts = () => Calendar.acts(meeting, provider);
            Object.assign(fields, {
                verb: Words.verb(joins ? "verb:calendar:join" : "verb:calendar:see"),
                later: joins,
                stays: !joins,
                run: joins ? () => Calendar.join(meeting)
                    : () => provider.launcher?.enter(Calendar.seeing(meeting, provider, row.name)),
                verbs: acts,
                targets: undefined
            });
        }
        return from.thing(row, key, fields);
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
