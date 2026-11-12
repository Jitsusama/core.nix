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
// A meeting's Return and most of what's inside it are still Calendar.qml's
// (Calendar.row's, as Now and Meet show it): Return joins its call from
// ten minutes before it starts, and otherwise steps in to Show in
// Calendar. Its Guests and Attachments are the rows the daemon serves
// inside it (src/calendar_inside.rs), each guest People's person and each
// document Documents' own row where they hold one (ready-rows.md's
// lends), and Move the Meeting, its later times and the row asking Joel
// to be sure of each. A time typed inside Move is still read by
// Calendar.js (`typed` on Move's row) till the daemon answers it as a
// search within Move's row, and its other acts stay the shell's till
// call-transcripts serves Open the Transcript, both by Sun 15 Nov; then
// every act inside a meeting is the daemon's.
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
        if (row.kind === "move")
            return provider.moving(from, row, key);
        const meeting = provider.meetingOf(row.held?.id ?? "");
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
            const acts = () => Calendar.acts(meeting, provider)
                .map(act => provider.servedInside(from, key, act) ?? act);
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

    // The meeting held as `id`, coming or held lately, or null.
    function meetingOf(id) {
        return Calendar.meeting(id) ?? Calendar.had.find(each => each.id === id) ?? null;
    }

    // Move the Meeting as the daemon serves it, stepping in to its later
    // times, with a time typed there read by Calendar.js and made as
    // Calendar.qml's moving() makes one, till the daemon answers it. It's
    // made here as every row under a meeting is, so it's the meeting
    // component's (Launcher.qml's components).
    function moving(from, row, key) {
        const meeting = provider.meetingOf(from.served(row.parent)?.held?.id ?? "");
        return from.thing(row, key, {
            component: "meeting",
            typed: letters => {
                const time = meeting ? Calendar.typed(letters, meeting) : null;
                return time === null ? [] : [Calendar.movingTo(meeting, provider,
                    Calendar.dated({ start: time.start, end: time.end, allDay: false }),
                    time.start, time.end)];
            }
        });
    }

    // The row the daemon serves inside the meeting at `key` in place of
    // the shell's `act`, for Guests and Attachments, or null for an act
    // it doesn't yet serve whole, or one it served none of.
    readonly property var servedActs: ({
        "verb:calendar:guests": "guests",
        "verb:calendar:attachments": "attachments",
        "verb:calendar:move": "move"
    })
    function servedInside(from, key, act) {
        const id = provider.servedActs[act?.key];
        return id ? from.rowAt(`${key}:${id}`) : null;
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
