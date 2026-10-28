// Arrivals: what comes true by itself, not by Joel's keys, is said once as
// a notice, so he knows the moment it happens rather than when he next
// looks at the corner (docs/design.md, the margin). A piece that `arrives`
// (a camera opened by a program, the battery running low) sends its hint
// as the notice's summary and its context as its message, signed with its
// program's name (`app`) and so its glyph, carrying its actions; and again,
// urgent, when it comes to need him now. While the card is up the margin
// leaves its glyph out (Margin.qml), and the card is never counted missed,
// since the glyph says it while it holds. What was true as the shell
// started, or after a restart, isn't news. One that `withdraws` takes its
// notice back, read, once it's no longer true, so a meeting's notice
// doesn't outlast the meeting starting.
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Io
import QtQuick

Scope {
    id: root

    // The launcher, which an action may open.
    required property var launcher

    // Whether the pieces have settled since the shell started: their
    // files and services are read in the first moments, which is no
    // arrival.
    property bool settled: false
    // What was true at the last change, by mark, with ":now" for what
    // needed him now.
    property var before: []
    // What's come true and is yet to be said.
    property var due: []

    readonly property var arriving: Facts.all
        .filter(fact => fact.piece && fact.piece.arrives && fact.piece.present
            && fact.piece.active)
    readonly property var standing: arriving
        .map(fact => fact.entry.mark + (fact.piece.now ? ":now" : ""))

    onStandingChanged: {
        if (settled) {
            for (const fact of arriving) {
                const key = fact.entry.mark + (fact.piece.now ? ":now" : "");
                if (!before.includes(key))
                    due.push(fact);
            }
        }
        before = standing;
        if (due.length > 0)
            Qt.callLater(root.sayDue);
        withdrawing.restart();
    }

    // A notice goes, read, a moment after what it said stops being true,
    // the moment letting a piece's bindings catch up, as they do a moment
    // after what made it true. Its mark rides on it as a hint.
    Timer {
        id: withdrawing
        interval: 1000
        onTriggered: {
            const marks = root.arriving.map(fact => fact.entry.mark);
            for (const id in Notices.live) {
                const notice = Notices.live[id];
                const mark = notice.hints["x-arrival"];
                if (mark !== undefined && !marks.includes(mark)) {
                    Notices.mark(notice.id, true);
                    notice.dismiss();
                }
            }
        }
    }

    // A piece's words follow what made it true a moment later, as the
    // rest of its bindings catch up, so it's said once they have, if it's
    // still true.
    function sayDue() {
        const facts = due;
        due = [];
        for (const fact of facts) {
            if (fact.piece.active)
                say(fact.piece, fact.entry.mark);
        }
    }

    // Long enough for every piece's first reading, short enough that a
    // camera opened just after the shell starts is still said.
    Timer {
        interval: 5000
        running: true
        onTriggered: {
            root.settled = true;
            if (Options.reports)
                Report.say("settles arrivals");
        }
    }

    // Says a piece as a notice, its actions offered by their place, and
    // marked as its mark's when it's to be taken back.
    function say(piece, mark) {
        const actions = piece.actions;
        notice.createObject(root, {
            actions,
            command: ["notify-send", `--app-name=${piece.app}`]
                .concat(piece.now ? ["--urgency=critical"] : [])
                .concat(piece.withdraws ? [`--hint=string:x-arrival:${mark}`] : [])
                .concat(actions.map((action, index) => `--action=${index}=${action.name}`))
                .concat([piece.hint], piece.context === "" ? [] : [piece.context])
        });
        if (Options.reports)
            Report.say(`arrives ${piece.title}`);
    }

    // notify-send waits while it offers actions, then prints the one Joel
    // chose.
    Component {
        id: notice

        Process {
            id: asking

            property var actions: []

            running: true
            stdout: SplitParser {
                onRead: line => {
                    const said = line.trim();
                    const chosen = /^\d+$/.test(said) ? asking.actions[Number(said)] : null;
                    if (chosen)
                        chosen.run({ launcher: root.launcher });
                }
            }
            onExited: asking.destroy()
        }
    }
}
