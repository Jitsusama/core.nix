pragma Singleton
pragma ComponentBehavior: Bound
// Tuple's people, for People's Tuple source (TupleSource.qml beside
// this): everyone Tuple lists as a contact,
// as tuple-people says them (people.rs), and how those Tuple knows stand.
//
// Tuple's own `contacts follow` says every contact again, over a megabyte
// of them, each time anyone's presence changes; the helper reads all of
// that so the shell doesn't, and says only what changed: everyone, which
// in practice is once, and how those Tuple knows stand, each time one
// does. It's followed once the launcher first opens (`follow`), and again
// as it next opens after it ended, as it does when Tuple's daemon stops.
// Any tuple command starts a daemon, so the helper runs Tuple only while
// its daemon answers, and a stopped Tuple stays stopped. Once the follow
// ends, how anyone stands is no longer known, so only who's a favourite or
// recent is kept.
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    // Everyone, each { id, name, email }, and who they are by id.
    property var everyone: []
    property var people: ({})
    // How those Tuple knows stand, by id: { status, favourite, recent }.
    property var standing: ({})

    // How each stands, in words and in the order of who can pair first.
    // Tuple says "unknown" of anyone not a favourite, which says nothing.
    readonly property var statuses: ({
        online: { said: "available", order: 0 },
        busy: { said: "in a call", order: 1 },
        unavailable: { said: "away", order: 2 },
        offline: { said: "offline", order: 3 }
    })

    function how(stands) {
        return statuses[stands ? stands.status : ""] ?? { said: "", order: 4 };
    }

    // Tuple finds someone by their email; the id Tuple knows them by is
    // the one left when they have none.
    function who(person) {
        return person.email || String(person.id);
    }

    // How they stand now, or null for one Tuple doesn't know.
    function standingOf(person) {
        return standing[person.id] ?? null;
    }

    // Calls one who isn't in a call and joins one who is.
    function pair(person) {
        const busy = standingOf(person)?.status === "busy";
        Niri.spawn(["tuple", "call", busy ? "join" : "start", who(person)]);
    }

    // Marks them a favourite or not at once, as Tuple will once it's told,
    // so their row moves now rather than when Tuple next says.
    function favour(person, wanted) {
        Niri.spawn(["tuple", "contacts", wanted ? "favorite" : "unfavorite", who(person)]);
        const now = Object.assign({ status: "unknown", recent: false }, standingOf(person),
            { favourite: wanted });
        const next = Object.assign({}, standing);
        if (now.favourite || now.recent || now.status !== "unknown")
            next[person.id] = now;
        else
            delete next[person.id];
        standing = next;
    }

    // How many Tuple knows can pair now.
    readonly property int available: Object.values(standing)
        .filter(stands => stands.status === "online").length

    // Each line says either everyone or how those Tuple knows stand.
    function heard(line) {
        let said;
        try {
            said = JSON.parse(line);
        } catch (error) {
            return;
        }
        if (Array.isArray(said?.people)) {
            const byId = {};
            const all = [];
            for (const [id, name, email] of said.people) {
                const person = { id: id, name: name, email: email };
                byId[id] = person;
                all.push(person);
            }
            root.people = byId;
            root.everyone = all;
        } else if (Array.isArray(said?.standing)) {
            const stood = {};
            for (const [id, status, favourite, recent] of said.standing)
                stood[id] = { status: status, favourite: favourite, recent: recent };
            root.standing = stood;
        }
    }

    // Whether the helper last found Tuple's daemon stopped.
    property bool stopped: false

    // What the helper says when no daemon answers (people.rs).
    readonly property int notRunning: 69

    function ended(code) {
        const kept = {};
        for (const id in standing) {
            const stands = standing[id];
            if (stands.favourite || stands.recent)
                kept[id] = { status: "unknown", favourite: stands.favourite, recent: stands.recent };
        }
        standing = kept;
        stopped = code === notRunning;
    }

    // Follows Tuple's contacts unless it already is, as the launcher opens.
    function follow() {
        if (!stream.running)
            stream.running = true;
    }

    // The helper's process, for a test to stop as a restart of the shell
    // would.
    readonly property Process following: stream

    Process {
        id: stream
        // Tuple's daemon listens here, as tuple-0 for its first account.
        command: ["tuple-people", "tuple", `${Quickshell.env("XDG_RUNTIME_DIR")}/tuple-0`]
        stdout: SplitParser {
            onRead: line => root.heard(line)
        }
        onStarted: root.stopped = false
        onExited: code => root.ended(code)
    }
}
