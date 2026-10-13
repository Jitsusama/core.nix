// Wi-Fi, as wifid tells it over varlink on the socket systemd holds for it:
// every card, the saved networks and those in the air, sent whole whenever
// any of it changes, so the shell asks nothing to keep up. Telling wifid
// what to do goes over a second connection, answered in order, and a join
// over one of its own, since it answers step by step until the card is done
// and would hold up everything asked after it. wifid starting again closes
// them all, and each comes back a moment later.
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

    // The card on a network, and one whose network wants a sign-in.
    readonly property var joined: cards.find(card => card.network) ?? null
    readonly property var signingIn: cards.find(card => card.reach === "signIn") ?? null

    // The join under way, as { name, stage }, and how the last one ended:
    // the stage it reached, or why it couldn't, as wifid names it.
    property var joining: null
    signal joinEnded(string name, string stage, string refused)

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

    function scan(): void {
        call("Scan", {});
    }

    function check(): void {
        call("Check", {});
    }

    function leave(name: string): void {
        call("Leave", { name: name });
    }

    function forget(name: string): void {
        call("Forget", { name: name });
    }

    function trust(name: string, trusted: bool): void {
        call("Trust", { name: name, trusted: trusted });
    }

    function turn(on: bool): void {
        call("Radio", { on: on });
    }

    // A join already under way gives way to a new one, as wifid gives it
    // way.
    // Joins a network, with its password when it asks one, and the account
    // ({ method, identity, serverDomain }) where it asks for one.
    function join(name: string, password: var, account: var): void {
        const parameters = { name: name };
        if (password !== undefined && password !== null)
            parameters.password = password;
        if (account !== undefined && account !== null)
            parameters.account = account;
        joining = { name: name, stage: "joining" };
        joinCall = message("Join", parameters, true);
        if (joins.connected)
            joins.connected = false;
        else
            joins.connected = true;
    }

    property var waiting: []
    property var answers: []
    property string joinCall: ""

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

    Socket {
        id: joins
        path: root.path
        onConnectionStateChanged: {
            if (connected && root.joinCall !== "") {
                write(root.joinCall);
                flush();
                root.joinCall = "";
            } else if (!connected && root.joinCall !== "") {
                // A join given way to: this one starts on a new connection,
                // once the socket has finished closing the last.
                Qt.callLater(() => joins.connected = true);
            } else if (!connected && root.joining) {
                const name = root.joining.name;
                root.joining = null;
                root.joinEnded(name, "", "Failed");
            }
        }
        parser: SplitParser {
            splitMarker: "\u0000"
            onRead: data => {
                const reply = JSON.parse(data);
                const name = root.joining ? root.joining.name : "";
                if (reply.error) {
                    root.joining = null;
                    root.joinEnded(name, "", root.refusal(reply.error));
                    joins.connected = false;
                    return;
                }
                const stage = reply.parameters.stage;
                if (reply.continues) {
                    root.joining = { name: name, stage: stage };
                    return;
                }
                root.joining = null;
                root.joinEnded(name, stage, "");
                joins.connected = false;
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
