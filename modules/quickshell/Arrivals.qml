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
// doesn't outlast the meeting starting. One that tells something new
// while it's true (`telling`), as a meeting does at each of its reminders
// and as it starts, is said again, its earlier notice taken back first, so
// one of its cards is up at a time: while its last card is still open,
// that card is said again in place, by its id, and raised again, so it's
// one card that shows anew rather than two saying different things. Whose
// words change while it tells the same, as the tokens' minutes count
// down, has its card said again in place too, without rising; one Joel
// has put away stays away until it tells something new.
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
    // What was true at the last change, by mark, with what each was
    // telling and ":now" for what needed him now.
    property var before: []
    // What's come true and is yet to be said.
    property var due: []
    // What each mark's notice said last, by mark: how it stood (`key`),
    // its words, the id the server gave it, once heard, and the
    // notify-send that answers its actions.
    property var told: ({})

    readonly property var arriving: Facts.all
        .filter(fact => fact.piece && fact.piece.arrives && fact.piece.present
            && fact.piece.active)
    readonly property var standing: arriving.map(fact => root.key(fact))
    // What each says, beside how it stands, so a change of words alone is
    // heard.
    readonly property var wording: arriving.map(fact => root.key(fact) + "\n" + root.words(fact.piece))

    onWordingChanged: {
        if (settled)
            Qt.callLater(root.follow);
    }

    // What a notice of a piece says, as one string to compare.
    function words(piece) {
        return JSON.stringify([piece.hint, root.message(piece)]
            .concat(piece.actions.map(root.offered)));
    }

    // What a piece's notice says beneath its hint.
    function message(piece) {
        return piece.message ?? piece.context;
    }

    // Where an act goes, too, since a button can't say it beside its name
    // as a row does: three sessions' Go To differ by it.
    function offered(action) {
        return action.side ? `${action.name} \u00b7 ${action.side}` : action.name;
    }

    // Says again in place each notice whose piece tells the same but in
    // new words, while its card is still open.
    function follow() {
        for (const fact of arriving) {
            const was = told[fact.entry.mark];
            if (was !== undefined && was.id !== 0 && was.key === root.key(fact)
                && was.words !== root.words(fact.piece) && String(was.id) in Notices.live)
                say(fact, was);
        }
    }

    // How a piece that's true stands: its mark, what it's telling when
    // it tells anything, and whether it needs him now.
    function key(fact) {
        return fact.entry.mark + (fact.piece.telling === "" ? "" : `:${fact.piece.telling}`)
            + (fact.piece.now ? ":now" : "");
    }

    onStandingChanged: {
        if (settled) {
            for (const fact of arriving) {
                if (!before.includes(root.key(fact)))
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
            for (const mark in root.told) {
                if (!marks.includes(mark))
                    delete root.told[mark];
            }
        }
    }

    // A piece's words follow what made it true a moment later, as the
    // rest of its bindings catch up, so it's said once they have, if it's
    // still true. Those bindings move one at a time, so a piece can pass
    // through a way of standing it never meant and be due twice; it's
    // said once, as it stands now. One whose last card the server is yet
    // to give an id is said once it has.
    function sayDue() {
        const facts = due;
        due = [];
        const saying = {};
        for (const fact of facts) {
            const mark = fact.entry.mark;
            if (mark in saying || !fact.piece.active)
                continue;
            saying[mark] = true;
            const was = told[mark];
            if (was !== undefined && was.id === 0 && was.asking) {
                was.behind = true;
                continue;
            }
            if (was !== undefined && was.id !== 0 && String(was.id) in Notices.live) {
                say(fact, was, true);
                continue;
            }
            if (fact.piece.telling !== "")
                takeBack(fact.entry.mark);
            say(fact, null);
        }
    }

    // Takes back, read, the notice a mark's piece said last, before it's
    // said anew, found by the mark it carries.
    function takeBack(mark) {
        for (const id in Notices.live) {
            const notice = Notices.live[id];
            if (notice.hints["x-arrival"] === mark) {
                Notices.mark(notice.id, true);
                notice.dismiss();
            }
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
    // marked as its mark's when it's to be taken back; or, given what its
    // notice said last, says it again in place of that one, whose
    // notify-send then gives way to this one's, raised again once the
    // server has it when it's `raising`.
    function say(fact, was, raising) {
        const piece = fact.piece;
        const mark = fact.entry.mark;
        const actions = piece.actions;
        const asking = notice.createObject(root, {
            actions,
            mark,
            raising: raising === true,
            command: ["notify-send", "--print-id", `--app-name=${piece.app}`]
                .concat(was === null ? [] : [`--replace-id=${was.id}`])
                .concat(piece.now ? ["--urgency=critical"] : [])
                .concat(piece.withdraws ? [`--hint=string:x-arrival:${mark}`] : [])
                .concat(actions.map((action, index) => `--action=${index}=${root.offered(action)}`))
                .concat([piece.hint], root.message(piece) === "" ? [] : [root.message(piece)])
        });
        told[mark] = {
            key: root.key(fact),
            words: root.words(piece),
            id: was === null ? 0 : was.id,
            asking
        };
        if (was !== null) {
            // Gone already when its notify-send has exited.
            if (was.asking)
                was.asking.running = false;
            Notices.reword(was.id, piece.hint, root.message(piece));
        }
        if (Options.reports)
            Report.say(`${was === null ? "arrives" : "rewords"} ${piece.title}`);
    }

    // notify-send says the id the server gave the notice, then waits while
    // it offers actions and prints the one Joel chose.
    Component {
        id: notice

        Process {
            id: asking

            property var actions: []
            property string mark: ""
            property bool raising: false
            property bool heard: false

            running: true
            stdout: SplitParser {
                onRead: line => {
                    const said = line.trim();
                    if (!asking.heard) {
                        asking.heard = true;
                        const was = root.told[asking.mark];
                        if (was !== undefined && was.asking === asking && /^\d+$/.test(said)) {
                            was.id = Number(said);
                            if (asking.raising)
                                Notices.raise(was.id);
                            const fact = root.arriving
                                .find(one => one.entry.mark === asking.mark);
                            // A step that came before the id is said now,
                            // and so are words that changed.
                            if (was.behind && fact && was.key !== root.key(fact))
                                root.say(fact, was, true);
                            else
                                Qt.callLater(root.follow);
                        }
                        return;
                    }
                    const chosen = /^\d+$/.test(said) ? asking.actions[Number(said)] : null;
                    if (chosen)
                        chosen.run({ launcher: root.launcher });
                }
            }
            onExited: asking.destroy()
        }
    }
}
