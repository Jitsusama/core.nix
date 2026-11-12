// The card of Joel's line to the one standing at Agents' top, the void
// above all (VKCF3Z, 2026-11-12 [a17c2491]): the launcher closes as ever
// on Return, and a quiet card from it says his line was sent; when the
// talk is replied to, the same card says the reply, answered, in place.
// Clicked away it's gone; its default action opens Agents, where the
// whole talk reads. Only his own line's reply makes one: what the void
// says to anyone else is never carded. Told.js says what each card says.
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

    // Each card up or waiting on its answer, by the address it went to:
    // { address, from, line, since, said, reply, id, asking }.
    property var cards: ({})

    // Said again whenever a talk's line comes, the reply among them.
    readonly property var heard: AgentSources.talks
    onHeardChanged: follow()

    // Joel's line to `address`, sent by the name it stands at the top by.
    function sent(address, from, line) {
        const was = cards[address];
        if (was?.asking)
            was.asking.running = false;
        cards[address] = { address, from, line, since: Date.now(), said: "sent", reply: "",
            id: 0, asking: null };
        say(cards[address], false);
    }

    // Each card waiting on its answer takes it once the talk has one whole,
    // in place of the sent one while that's up, or as a card of its own once
    // he's put that away; a reply given again, at the run's end, replaces
    // an answered card only while it's up.
    function follow() {
        for (const address in cards) {
            const card = cards[address];
            const reply = Told.replyOf(Told.talkOf(root.talksOf(address), card.line, card.since));
            if (reply === "" || reply === card.reply)
                continue;
            if (card.said === "answered" && card.asking === null)
                continue;
            if (card.asking !== null && card.id === 0) {
                card.behind = true;
                continue;
            }
            card.said = "answered";
            card.reply = reply;
            say(card, card.asking !== null);
        }
    }

    function say(card, replacing) {
        const said = Words.of("verb:agent:talk").said ?? {};
        const was = card.asking;
        card.asking = asking.createObject(root, {
            address: card.address,
            command: Told.command({
                id: replacing ? card.id : 0,
                from: card.from,
                said: card.said,
                open: String(said.open ?? ""),
                line: card.line,
                reply: Told.cut(card.reply, String(said.more ?? ""))
            })
        });
        if (was)
            was.running = false;
    }

    // notify-send says the id the server gave the card, then waits while
    // it's up and says its default action if Joel takes it.
    Component {
        id: asking

        Process {
            id: process

            property string address: ""
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
                            if (card.behind) {
                                card.behind = false;
                                Qt.callLater(root.follow);
                            }
                        }
                        return;
                    }
                    if (said === "default")
                        root.launcher.chordInto("agents");
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
