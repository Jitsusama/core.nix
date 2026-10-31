// Slack as one of People's sources (PersonSource.qml): Write in Slack for
// anyone Slack knows by one of their addresses, in the harness's Slack
// sign-in's workspace, first of the ways to reach them, and how they are
// there: active, away or in a huddle, with their status's emoji, in their
// note, and their status and presence in the drawer (VKCF3Z, 42b57b7d,
// 67f6cbac and 3bbad443). It knows nobody by itself, since a workspace
// has thousands; it asks slack-people, a reader that never sends
// anything, about each person People lists as the launcher opens, ahead
// and paced, so who they are is known before Joel steps into them, and
// how they are as he steps in, again once what it heard is a minute old.
// The reader keeps who was found between runs, so a restart forgets
// nobody. Whoever Slack found it knows, with their time zone as a fact,
// which People follows with the time there.
//
// Write in Slack never arrives late or moves under the keys: for someone
// not yet looked up, it stands in its place dimmed, "looking in Slack",
// lighting there when Slack answers, or saying "not in Slack" until the
// launcher opens again. Someone already known not to be in Slack has no
// such row (VKCF3Z, 8a0f14ea). Slack refusing the sign-in stops the
// asking until the shell starts again; with no sign-in, nothing stands
// in waiting.
//
// Their direct messages with Joel are one conversation in their Recent,
// named by its newest message's first words, beside "Slack" and noted
// with when, among mail and meetings by that time; Return opens it in
// Slack, and inside are Open in Slack then Copy the Link, its drawer its
// last few messages (VKCF3Z, 42b57b7d). It's asked ahead too, so their
// Recent is there as Joel steps in rather than coming late.
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Slack.js" as Slack
import "../Ago.js" as Ago

PersonSource {
    id: source

    called: "Slack"

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
    // Whether there's a sign-in Slack takes: with none, or one refused,
    // nobody stands waiting on an answer that won't come.
    property bool asks: true
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

    people: Object.values(known).filter(said => said.id).map(said => ({
        emails: [said.email],
        name: said.name,
        facts: Slack.facts(said)
    }))

    // How each stands in Slack, by address, for their note: active, away
    // or in a huddle, and their status's emoji as its mark, which People
    // weighs with what other sources say (Person.standing).
    stands: {
        const by = {};
        for (const said of Object.values(known)) {
            if (said.id && (said.presence || said.emoji))
                by[said.email] = {
                    said: said.presence,
                    order: presenceOrder[said.presence] ?? 4,
                    mark: said.emoji
                };
        }
        return by;
    }
    readonly property var presenceOrder: ({
        active: 0,
        "in a huddle": 1,
        away: 2
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
    function recent(person, provider) {
        const said = Slack.talked(person.emails, known);
        return said ? [{ at: said.conversation.at, row: conversation(said, provider) }] : [];
    }

    // Slack's conversation row, the one every surface shows
    // (SlackConversation.qml), named by its newest message's first words.
    function conversation(said, provider) {
        const talk = said.conversation;
        return SlackConversation.row({
            id: talk.channel,
            name: talk.last !== "" ? talk.last : Words.name("slack:conversation"),
            opens: `slack://channel?team=${said.team}&id=${talk.channel}`,
            link: talk.link,
            side: source.called,
            note: Ago.ago(talk.at, Date.now()),
            drawer: Slack.drawn(said)
        }, fields => provider.thing(fields));
    }

    // Asks Slack about each of their addresses that's due (Slack.due).
    function ask(person) {
        const due = Slack.due(person.emails, known, {
            answered: answered,
            asking: asking,
            now: Date.now(),
            fresh: fresh
        });
        if (due.length === 0)
            return;
        if (!reader.running)
            reader.running = true;
        // Qt's JavaScript has no Object.fromEntries.
        const now = Date.now();
        const asked = Object.assign({}, asking);
        for (const email of due)
            asked[email] = now;
        asking = asked;
        for (const email of due)
            reader.write(`${email}\n`);
    }

    // As the launcher opens: whoever People lists and Slack hasn't yet
    // been asked about is asked about ahead, in the order they're listed,
    // and what a step-in found missing is forgotten.
    function woken() {
        if (Object.keys(missed).length > 0)
            missed = {};
        if (!asks || !place)
            return;
        const due = Slack.ahead(place.all.map(row => row.person), known, ahead);
        if (due.length === 0)
            return;
        for (const email of due)
            ahead[email] = true;
        if (!reader.running)
            reader.running = true;
        reader.write(due.map(email => `?${email}\n`).join(""));
    }

    function heard(line) {
        let said;
        try {
            said = JSON.parse(line);
        } catch (error) {
            return;
        }
        if (said?.signedOut || said?.signedIn === false) {
            // Signed out stops the reader; with no sign-in it keeps
            // reading the file, so signing in from pi is taken at once.
            if (said.signedOut)
                reader.running = false;
            // What was asked is answered, so the person made again for
            // it doesn't ask once more at once.
            const now = Date.now();
            for (const email of Object.keys(asking))
                answered[email] = now;
            asking = {};
            const stood = asks;
            asks = false;
            if (stood)
                place?.changed();
            return;
        }
        if (typeof said?.email !== "string")
            return;
        asks = true;
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
        if (!settling.running)
            settling.start();
    }

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
        onTriggered: source.settle()
    }

    Process {
        id: reader
        command: ["slack-people", source.settings.credentials ?? ""]
        stdinEnabled: true
        stdout: SplitParser {
            onRead: line => source.heard(line)
        }
    }
}
