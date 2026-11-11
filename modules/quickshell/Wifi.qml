// Wi-Fi, as wifid tells it over varlink on the socket systemd holds for it:
// every card, the saved networks and those in the air, sent whole whenever
// any of it changes, so the shell asks nothing to keep up. The launcher
// asks wifid's acts, joins among them, on its rows socket
// (launcher/Wireless.qml), where a network's drawer and parts are served
// too, while it's ahead; a check goes over a second connection, answered
// in order. wifid starting again closes both, and each comes back a moment
// later.
pragma Singleton
import Quickshell
import Quickshell.Io
import QtQuick

Singleton {
    id: root

    // As wifid last described them, and whether it has yet.
    property var cards: []
    property var saved: []
    property var nearby: []
    property bool radio: true
    property bool wired: false
    property bool described: false
    // How many looks for networks the cards have finished, which moves
    // once a look asked for is done, whatever it heard.
    property int looks: 0

    // The card on a network, and one whose network wants a sign-in.
    readonly property var joined: cards.find(card => card.network) ?? null
    readonly property var signingIn: cards.find(card => card.reach === "signIn") ?? null

    function isSaved(name: string): var {
        return saved.find(network => network.name === name) ?? null;
    }

    function isJoined(name: string): bool {
        return cards.some(card => card.network === name);
    }

    // Asks wifid something, handing `answer` its parameters and the error,
    // if it refused, with its own name and no interface's in front.
    function call(method: string, parameters: var, answer: var): void {
        waiting.push(message(method, parameters, false));
        answers.push(answer ?? null);
        if (calls.connected)
            send();
        else
            calls.connected = true;
    }

    function check(): void {
        call("Check", {});
    }

    property var waiting: []
    property var answers: []

    function message(method: string, parameters: var, more: bool): string {
        const call = { method: `io.jitsusama.Wifi.${method}` };
        // wifid's varlink library closes the connection on empty parameters
        // given to a method that takes none, so none are given.
        if (Object.keys(parameters).length > 0)
            call.parameters = parameters;
        if (more)
            call.more = true;
        return JSON.stringify(call) + "\u0000";
    }

    function send(): void {
        for (const call of waiting)
            calls.write(call);
        waiting = [];
        calls.flush();
    }

    function hear(described: var): void {
        cards = described.cards;
        saved = described.saved;
        nearby = described.nearby;
        radio = described.radio;
        wired = described.wired;
        looks = described.looks ?? 0;
        root.described = true;
    }

    function refusal(error: var): string {
        return error ? error.replace("io.jitsusama.Wifi.", "") : "";
    }

    readonly property string path: "/run/wifid/io.jitsusama.Wifi"

    Socket {
        id: describing
        path: root.path
        connected: true
        onConnectionStateChanged: {
            if (connected) {
                write(root.message("Describe", {}, true));
                flush();
            } else {
                comingBack.start();
            }
        }
        parser: SplitParser {
            splitMarker: "\u0000"
            onRead: data => {
                const reply = JSON.parse(data);
                if (!reply.error)
                    root.hear(reply.parameters);
            }
        }
    }

    Socket {
        id: calls
        path: root.path
        onConnectionStateChanged: {
            // Calls sent on a connection that closed are never answered, so
            // only the unsent keep a place in line.
            if (!connected)
                root.answers = root.answers.slice(root.answers.length - root.waiting.length);
            else if (root.waiting.length > 0)
                root.send();
        }
        parser: SplitParser {
            splitMarker: "\u0000"
            onRead: data => {
                const reply = JSON.parse(data);
                const answer = root.answers.shift();
                if (reply.error)
                    console.warn("wifid refused:", reply.error);
                if (answer)
                    answer(reply.parameters ?? {}, root.refusal(reply.error));
            }
        }
    }

    // wifid starting again closes the connection that hears it, which
    // comes back a moment later: systemd holds the socket meanwhile, so
    // connecting never fails while wifid is installed.
    Timer {
        id: comingBack
        interval: 1000
        onTriggered: describing.connected = true
    }
}
