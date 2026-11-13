// The card of Joel's line to the one standing at Agents' top, the void
// above all (VKCF3Z, 2026-11-12 [a17c2491]): the launcher closes as ever
// on Return, and a quiet card from it says his line was sent; when the
// talk is replied to, the same card says the reply, answered, in place.
// Clicked away it's gone; its default action opens Agents, where the
// whole talk reads. Only his own line's reply makes one: what the void
// says to anyone else is never carded. Where the void filed his line, it
// offers Take It Back, which tells the void to undo it and says, in
// place, that it was taken back (VKCF3Z, 2026-11-12 [244ed4fd]). The
// agents daemon takes sending it next, with the rest of Talk. Told.js
// says what each card says.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Told.js" as Told

Scope {
    id: root

    // The launcher, which the card's default action opens into Agents.
    required property var launcher
    // A session's talks by its address, as Orchestrators.qml folds them.
    required property var talksOf
    // Says Joel's words to the session at an address, as Talk does.
    required property var talk

    // Each card up or waiting on its answer, by the address it went to:
    // { address, from, line, since, said, heard, reply, undo, taking,
    // takenSince, told, id, asking }: `heard` the reply as the talk gave
    // it, `undo` the time the void takes it back by, `taking` the line
    // asking it to, while its answer is awaited, and `told` that answer
    // where it isn't the void's own "undone".
    property var cards: ({})

    // Said again whenever a talk's line comes, the reply among them.
    readonly property var heard: AgentSources.talks
    onHeardChanged: follow()

    // Joel's line to `address`, sent by the name it stands at the top by.
    function sent(address, from, line) {
        const was = cards[address];
        if (was?.asking)
            was.asking.running = false;
        cards[address] = { address, from, line, since: Date.now(), said: "sent", heard: "",
            reply: "", undo: "", taking: "", takenSince: 0, told: "", id: 0, asking: null };
        say(cards[address], false);
    }

    // Each card waiting on its answer takes it once the talk has one whole,
    // in place of the sent one while that's up, or as a card of its own once
    // he's put that away; a reply given again, at the run's end, replaces
    // an answered card only while it's up, and never one taken back.
    function follow() {
        for (const address in cards) {
            const card = cards[address];
            if (card.taking !== "") {
                followTaking(card);
                continue;
            }
            if (card.said === "taken" || card.said === "told")
                continue;
            const reply = Told.replyOf(Told.talkOf(root.talksOf(address), card.line, card.since));
            if (reply === "" || reply === card.heard)
                continue;
            if (card.said === "answered" && card.asking === null)
                continue;
            if (card.asking !== null && card.id === 0) {
                card.behind = true;
                continue;
            }
            const went = Told.went(reply);
            card.said = "answered";
            card.heard = reply;
            card.reply = went.reply;
            card.undo = went.undo;
            say(card, card.asking !== null);
        }
    }

    // Taken back, the card says the void's answer to that once it's given
    // whole, in place while it's up, or as a card of its own once he's put
    // it away.
    function followTaking(card) {
        const answer = Told.replyOf(Told.talkOf(root.talksOf(card.address), card.taking,
            card.takenSince));
        if (answer === "")
            return;
        Object.assign(card, Told.takenBy(answer), { taking: "" });
        say(card, card.asking !== null);
    }

    // Take It Back, from the card or its drawer: the void is told to undo
    // the line by the time it filed it at, as Joel's own line, and the act
    // goes from the card at once, so it's never sent twice (VKCF3Z,
    // 2026-11-12 [244ed4fd]).
    function take(card) {
        if (card.undo === "")
            return;
        card.taking = Told.undoing(card.undo);
        card.takenSince = Date.now();
        card.undo = "";
        root.talk(card.address, card.taking);
        Report.say(`told: took back ${card.taking}`);
        say(card, true);
    }

    function say(card, replacing) {
        const said = Words.of("verb:agent:talk").said ?? {};
        const was = card.asking;
        card.asking = asking.createObject(root, {
            address: card.address,
            began: Date.now(),
            command: Told.command({
                id: replacing ? card.id : 0,
                from: card.from,
                said: card.said,
                told: card.told,
                open: String(said.open ?? ""),
                undo: card.undo,
                takes: String(said.take ?? ""),
                line: card.line,
                reply: Told.cut(card.reply, String(said.more ?? ""))
            })
        });
        if (was)
            was.running = false;
    }

    // notify-send says the id the server gave the card, then waits while
    // it's up and says its default action, or Take It Back, if Joel takes
    // it.
    Component {
        id: asking

        Process {
            id: process

            property string address: ""
            property real began: 0
            property bool heard: false

            running: true
            stdout: SplitParser {
                onRead: line => {
                    const said = line.trim();
                    const card = root.cards[process.address];
                    if (!process.heard) {
                        process.heard = true;
                        if (card?.asking === process && /^\d+$/.test(said)) {
                            card.id = Number(said);
                            // How long Return, or the reply, took to reach
                            // the notices as a card, held to Joel's 50 ms.
                            console.info(`told: the ${card.said} card reached the notices `
                                + `${Date.now() - process.began} ms after it was asked`);
                            if (card.behind) {
                                card.behind = false;
                                Qt.callLater(root.follow);
                            }
                        }
                        return;
                    }
                    if (said === "default")
                        root.launcher.chordInto("agents");
                    else if (said === "take" && card)
                        root.take(card);
                }
            }
            onExited: {
                const card = root.cards[process.address];
                if (card?.asking === process)
                    card.asking = null;
                process.destroy();
            }
        }
    }
}
