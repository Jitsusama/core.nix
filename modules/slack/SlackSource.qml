// Slack as one of People's sources (PersonSource.qml): Write in Slack for
// anyone Slack knows by one of their addresses, in the harness's Slack
// sign-in's workspace, first of the ways to reach them, and how they are
// there: active, away or in a huddle, with their status's emoji, in their
// note, and their status and presence in the drawer (VKCF3Z, 42b57b7d,
// 67f6cbac and 3bbad443). It knows nobody by itself, since a workspace
// has thousands; it asks the people daemon, which runs slack-people, a
// reader that never sends anything, about each person People lists as
// the launcher opens, ahead and paced, so who they are is known before
// Joel steps into them, and how they are as he steps in, again once what
// it heard is a minute old. The shell runs no reader of its own, so Slack
// is asked once for everyone who asks (the daemon's own lines were a
// second slack-people asking the same, 2026-11-02). The reader keeps who
// was found between runs, so a restart forgets nobody. Who Slack found,
// and how each stands there, People reads from the daemon (`daemonHolds`)
// for those it lists and those a search finds, never from a copy held
// here: merged from one, a new shell's first letter made a row for each
// the daemon neither listed nor found (Joel's journal, 2026-11-13). What
// it heard is kept here only for the ways to reach them and their
// conversation.
//
// Write in Slack never arrives late or moves under the keys: for someone
// not yet looked up, it stands in its place dimmed, "looking in Slack",
// lighting there when Slack answers, or saying "no one at this address" until the
// launcher opens again. Someone already known not to be in Slack has no
// such row (VKCF3Z, 8a0f14ea). With Slack's sign-in refused or missing,
// or the daemon not answering, nothing stands in waiting.
//
// Their direct messages with Joel are one conversation in their Messages,
// named by its newest message's first words, beside "Slack" and noted
// with when, among mail by that time, and in their Waiting too while one
// owes the other an answer; Return opens it in
// Slack, and inside are Open in Slack then Copy the Link, its drawer its
// last few messages (VKCF3Z, 42b57b7d). It's asked ahead too, so their
// Messages are there as Joel steps in rather than coming late.
import qs
import Quickshell
import QtQuick
import "Slack.js" as Slack
import "../Ago.js" as Ago
import "../../Hearing.js" as Hearing

PersonSource {
    id: source

    called: "Slack"

    daemonHolds: true

    // What Slack said of each address asked about: { id, team, name,
    // zone, presence, status, emoji }, with no id for one Slack knows
    // nobody by.
    property var known: ({})
    // When each address was last asked about as Joel stepped in, and not
    // yet answered.
    property var asking: ({})
    // Each address asked about ahead, kept in place, since nothing is
    // drawn from it.
    property var ahead: ({})
    // Each address Slack was asked about as Joel stepped in and knows
    // nobody by, said so in place of Write in Slack until the launcher
    // opens again.
    property var missed: ({})
    // Whether Slack can be asked: heard answering, with a sign-in Slack
    // takes, since it was last heard unable to. Only Slack saying what it
    // knows of someone says so, since the daemon calls a reader answering
    // from the moment it starts, before it has said anything, and a
    // placeholder stood on that word would stay, dimmed, once Slack said
    // it was signed out (Steady.js). With none, nobody stands waiting on
    // an answer that won't come.
    property bool asks: false
    // When Slack was last heard unable to answer, by this machine's clock,
    // which the daemon's records are timed by too.
    property real quiet: 0
    // Everyone the daemon is watched for, by key, those asked ahead and
    // those stepped into, so how each is in Slack is heard as it changes.
    property var watched: []
    // How long a step-in waits on Slack's answer through the daemon,
    // which a look-up by search can take a few seconds for.
    readonly property int patience: 5000
    // When each address was last answered, kept in place rather than set
    // anew, since nothing is drawn from it and setting it would remake
    // everyone.
    property var answered: ({})
    // How long an answer, or an ask Slack never answered, holds before a
    // step-in asks again.
    readonly property int fresh: 60 * 1000
    // The newest answer for each address not yet taken, kept in place:
    // answers that come together, as a restart's kept people do, are
    // taken as one change, since each change makes everyone anew.
    property var pending: ({})

    // One it was asked about and knows nobody by, said with no id, is no
    // one of Slack's.
    function knows(entry) {
        return Boolean(entry.id);
    }

    // How one stands in Slack, from what Slack told the daemon of them,
    // for their note: active, away or in a huddle in People's words
    // (words.json), a presence it doesn't know said as it is, and their
    // status's emoji as its mark, which People weighs with what other
    // sources say (Person.standing). The daemon words those it lists and
    // finds the same way, from Slack's `stands` in its module.
    function stood(entry) {
        const presence = presences[entry.presence ?? ""];
        return {
            said: presence ? Words.of("people").said[presence.word]
                : String(entry.presence ?? ""),
            order: presence?.order ?? 4,
            mark: String(entry.emoji ?? ""),
            favourite: false,
            recent: false
        };
    }
    readonly property var presences: ({
        active: { word: "active", order: 0 },
        "in a huddle": { word: "huddle", order: 1 },
        away: { word: "away", order: 2 }
    })

    function acts(person, provider) {
        ask(person);
        const found = Slack.found(person.emails, known);
        const row = {
            key: "verb:person:slack",
            name: Words.name("verb:person:slack"),
            verb: Words.verb("verb:person:slack"),
            glyph: Words.glyph("verb:person:slack"),
            hue: "action",
            main: true,
            later: true
        };
        // Their message he hasn't answered is answered where it was said,
        // first of their acts (VKCF3Z, 6ea9cd26: "Reply in Slack").
        const talked = found ? Slack.talked(person.emails, known) : null;
        if (talked && Slack.owed(talked)) {
            const reply = "verb:person:slack:reply";
            return [provider.thing(Object.assign(row, {
                key: reply,
                name: Words.name(reply),
                verb: Words.verb(reply),
                glyph: Words.glyph(reply),
                run: () => Niri.spawn(["xdg-open", Slack.opens(talked)])
            }))];
        }
        if (found)
            return [provider.thing(Object.assign(row, {
                run: () => Niri.spawn(["xdg-open",
                    `slack://user?team=${found.team}&id=${found.id}`])
            }))];
        const waiting = Slack.waiting(person.emails, known, missed, asks);
        if (waiting === "")
            return [];
        // Dimmed, so Return does nothing until it lights.
        const said = Words.of("verb:person:slack").said ?? {};
        return [provider.thing(Object.assign(row, {
            gone: true,
            side: String(said[waiting] ?? ""),
            run: () => {}
        }))];
    }

    // Their conversation with Joel, when Slack found one.
    // Their direct messages are what they wrote each other, saying whether
    // Joel wrote last, as their page's Messages says (People.qml).
    function recent(person, provider) {
        const said = Slack.talked(person.emails, known);
        return said ? [{ at: said.conversation.at, row: conversation(said, provider),
            kind: "wrote", mine: Slack.wroteLast(said) }] : [];
    }

    // And they wait while one of them owes the other an answer
    // (Slack.owing), in their page's Waiting.
    function unread(person, provider) {
        const said = Slack.talked(person.emails, known);
        return said && Slack.owing(said, Date.now())
            ? [{ at: said.conversation.at, row: conversation(said, provider),
                what: source.called, mine: Slack.wroteLast(said) }] : [];
    }

    // The first of each for their drawer, with no row made: the
    // conversation while it waits (PersonSource's firsts).
    function firsts(person, provider) {
        const said = Slack.talked(person.emails, known);
        const talk = said?.conversation;
        return {
            met: null,
            wait: said && Slack.owing(said, Date.now()) ? {
                at: talk.at, name: talk.last !== "" ? talk.last : Words.name("slack:conversation"),
                what: source.called, mine: Slack.wroteLast(said), opens: Slack.opens(said)
            } : null
        };
    }

    // Slack's conversation row, the one every surface shows
    // (SlackConversation.qml), named by its newest message's first words.
    function conversation(said, provider) {
        const talk = said.conversation;
        return SlackConversation.row({
            id: talk.channel,
            name: talk.last !== "" ? talk.last : Words.name("slack:conversation"),
            opens: Slack.opens(said),
            link: talk.link,
            side: source.called,
            note: Ago.ago(talk.at, Date.now()),
            drawer: Slack.drawn(said)
        }, fields => provider.thing(fields));
    }

    // Asks Slack about each of their addresses that's due (Slack.due), as
    // a step-in of the daemon's, which answers with what it holds once
    // Slack has said or its patience is spent; what Slack says after
    // comes by the watch.
    function ask(person) {
        const due = Slack.due(person.emails, known, {
            answered: answered,
            asking: asking,
            now: Date.now(),
            fresh: fresh
        });
        if (due.length === 0)
            return;
        // Qt's JavaScript has no Object.fromEntries.
        const now = Date.now();
        const asked = Object.assign({}, asking);
        for (const email of due)
            asked[email] = now;
        asking = asked;
        const keys = due.map(email => `person:${email}`);
        watch(keys);
        // Asked even while Slack can't be, since a step-in is what tells
        // Slack's reader a sign-in came; the daemon's watch says how it
        // stands as soon as that changes.
        book.get(keys, patience, null);
    }

    // Watches these keys too, as well as those watched already.
    function watch(keys) {
        const fresh = keys.filter(key => !watched.includes(key));
        if (fresh.length > 0)
            watched = watched.concat(fresh);
    }

    // How Slack's reader stands, in the daemon's word: not to be asked
    // while it's signed out, refused, or not run at all (""). Starting or
    // answering says nothing of its sign-in, so only what Slack says of
    // someone after makes it askable again (took).
    function standsAs(word) {
        if (word === "starting" || word === "answering")
            return;
        const now = Date.now();
        quiet = now;
        // What was asked is answered, so the person made again for it
        // doesn't ask once more at once.
        for (const email of Object.keys(asking))
            answered[email] = now;
        if (Object.keys(asking).length > 0)
            asking = {};
        if (!asks)
            return;
        asks = false;
        place?.changed();
    }

    // As the launcher opens: whoever People lists and Slack hasn't yet
    // been asked about is asked about ahead, in the order they're listed,
    // and what a step-in found missing is forgotten.
    function woken() {
        if (Object.keys(missed).length > 0)
            missed = {};
        // Asked ahead even while Slack can't be, since its answers are
        // what say it can; each is asked once while the shell runs.
        if (!place)
            return;
        const due = Slack.ahead(place.all.map(row => row.person), known, ahead);
        if (due.length === 0)
            return;
        for (const email of due)
            ahead[email] = true;
        // Watching them asks the daemon to ask Slack ahead about each.
        watch(due.map(email => `person:${email}`));
    }

    // What the daemon said of these keys, Slack's part of each taken as
    // Slack's answer.
    function took(keys) {
        for (const key of keys) {
            const record = book.held[key];
            const said = Slack.fromRecord(record, settings.name);
            if (!said)
                continue;
            // Slack said this since it was last unable to: it's signed in.
            if (!asks && Number(record.heard ?? 0) > quiet) {
                asks = true;
                place?.changed();
            }
            heard(said);
        }
    }

    function heard(said) {
        // Only a step-in's answer says how they are, so only it holds the
        // next step-in back a minute.
        const steppedIn = asking[said.email] !== undefined;
        if (steppedIn) {
            answered[said.email] = Date.now();
            const left = Object.assign({}, asking);
            delete left[said.email];
            asking = left;
            if (!said.id)
                missed = Object.assign({}, missed, { [said.email]: true });
        }
        pending[said.email] = said;
        Hearing.putOff(settled);
        if (!settling.running)
            settling.start();
    }
    // The burst is told as from its first line (Hearing.js).
    readonly property var settled: Hearing.carried(() => source.settle())

    // Takes the answers that came together as one change, or none when
    // they're what was known, so nobody is made anew.
    function settle() {
        const next = Slack.settled(known, pending);
        pending = {};
        if (!next)
            return;
        known = next;
        place?.changed();
    }

    Timer {
        id: settling
        // Started by the first of a burst, never put off by the rest, so
        // a step-in's answer lights within a quarter of a second.
        interval: 250
        onTriggered: source.settled.tell()
    }

    Daemon {
        id: book

        socket: `${Quickshell.env("XDG_RUNTIME_DIR") ?? ""}/daemons/people.sock`
        kind: "person"
        // Only those Slack's asked about, never everyone the daemon holds.
        watching: ({ keys: [] })
        ahead: source.watched
        onHeard: keys => source.took(keys)
        // How Slack stands, as the daemon says when it answers, and that
        // it's not to be asked while the daemon isn't there.
        onSourcesChanged: source.standsAs(book.sources?.[source.settings.name] ?? "")
        onStateChanged: {
            if (book.state !== "answering")
                source.standsAs("");
        }
    }
}
