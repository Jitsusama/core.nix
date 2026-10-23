// What to open something with, asked in the launcher for the portal's
// chooser (open-with, src/main.rs), which asks here when a program opens a
// file or a link more than one app could open and the portal can't tell
// which. The launcher opens holding the apps the portal offers, the one
// picked last time first, under "open notes.txt with?". Return opens it
// with the app picked; Escape, or closing the launcher, opens nothing. The
// answer goes back on the socket the chooser listens on, as the app's id,
// or an empty line for none.
//
// Asks wait their turn, oldest first, as askpass's do. A collection only
// while one is waiting, it's never at home or in a search otherwise. An ask
// is known by its socket, which nothing changes, since the apps it offers
// can change while it waits, and a row or the launcher may still hold the
// ask as it was before.
import qs
import Quickshell
import Quickshell.Io
import QtQuick

Provider {
    id: provider

    what: "program"

    // Who's waiting, oldest first: { reply, what, kind, last, offered }.
    property var asks: []
    readonly property var asking: asks.length > 0 ? asks[0] : null
    // The socket of the ask the launcher holds, which closing it refuses.
    property string shown: ""

    collection: asking === null ? null : ({
        noun: "apps",
        title: "Open With",
        searched: false,
        glyph: String.fromCodePoint(0xf03cc),
        ask: asking.what === "" ? "open it with?" : `open ${named(asking.what)} with?`,
        empty: "Nothing here opens it."
    })

    // A link or a path, said by what ends it, so the ask stays short.
    function named(what) {
        const plain = what.replace(/^file:\/\//, "").replace(/[?#].*$/, "").replace(/\/+$/, "");
        const last = plain.slice(plain.lastIndexOf("/") + 1);
        try {
            return decodeURIComponent(last) || what;
        } catch (error) {
            return last || what;
        }
    }

    // The apps offered, the last one picked first, each as its desktop entry
    // names it; one with no entry left out, since nothing could open it.
    function offered(ask) {
        const ids = ask.offered.slice();
        const at = ids.indexOf(ask.last);
        if (at > 0)
            ids.unshift(ids.splice(at, 1)[0]);
        return ids
            .map(id => ({ id: id, entry: DesktopEntries.byId(id) }))
            .filter(found => found.entry !== null)
            .map(found => thing({
                key: `open-with:${found.id}`,
                verb: "open",
                name: found.entry.name,
                glyph: programGlyph(found.id),
                hue: "program",
                side: found.id === ask.last ? "last time" : found.entry.genericName.toLowerCase(),
                run: () => provider.settle(ask.reply, found.id)
            }));
    }

    readonly property var choices: asking === null ? [] : offered(asking)

    function everything() {
        return choices.slice();
    }

    function counted() {
        return `${choices.length} apps`;
    }

    // Says the pick, or none, on the ask's socket, and asks the next.
    function settle(reply, id) {
        if (!asks.some(other => other.reply === reply))
            return;
        asks = asks.filter(other => other.reply !== reply);
        if (shown === reply)
            shown = "";
        // It connects only once made: a local socket can connect, say and
        // close at once, and an object still being made can't be destroyed.
        const answer = answering.createObject(provider, { path: reply, said: `${id}\n` });
        answer.connected = true;
        if (asking !== null)
            Qt.callLater(provider.showNext);
    }

    // Holds the oldest ask in the launcher, unless it already does.
    function showNext() {
        if (asking === null || (shown === asking.reply && launcher.open))
            return;
        shown = asking.reply;
        launcher.into(settings.name);
    }

    // One answer: connected, said, then gone.
    Component {
        id: answering

        Socket {
            id: answer

            required property string said

            onConnectionStateChanged: {
                if (answer.connected) {
                    answer.write(answer.said);
                    answer.flush();
                    answer.connected = false;
                } else {
                    answer.destroy();
                }
            }
            onError: answer.destroy()
        }
    }

    // Closing the launcher on an ask, or leaving it for something else,
    // opens nothing. The launcher closes before it runs the row picked, so
    // the refusal waits a turn, by when a pick has answered the ask.
    Connections {
        target: provider.launcher

        function onOpenChanged() {
            if (!provider.launcher.open && provider.shown !== "")
                Qt.callLater(provider.refuse, provider.shown);
        }
    }

    function refuse(reply) {
        if (!launcher.open && shown === reply)
            settle(reply, "");
    }

    IpcHandler {
        target: "open-with"

        // Asks what to open what with, for the chooser listening on reply,
        // one of the apps offered (ids split by ";"), last the one picked
        // before. Only a socket the chooser makes is answered.
        function ask(reply: string, what: string, kind: string, last: string, offered: string): void {
            const runtime = Quickshell.env("XDG_RUNTIME_DIR");
            const name = reply.startsWith(`${runtime}/open-with/`)
                ? reply.slice(runtime.length + "/open-with/".length) : "";
            if (!/^[0-9]+\.sock$/.test(name) || provider.asks.some(other => other.reply === reply))
                return;
            const first = provider.asking === null;
            provider.asks = provider.asks.concat([{
                reply: reply,
                what: what,
                kind: kind,
                last: last,
                offered: offered.split(";").filter(id => id !== "")
            }]);
            if (first)
                Qt.callLater(provider.showNext);
        }

        // The apps that open it changed while it waits.
        function update(reply: string, offered: string): void {
            provider.asks = provider.asks.map(other => other.reply !== reply ? other
                : Object.assign({}, other, { offered: offered.split(";").filter(id => id !== "") }));
        }

        // Stops asking, once the program gave up or the chooser stopped
        // waiting.
        function withdraw(reply: string): void {
            const withdrawn = provider.shown === reply;
            provider.asks = provider.asks.filter(other => other.reply !== reply);
            if (!withdrawn)
                return;
            provider.shown = "";
            if (provider.asking !== null)
                provider.showNext();
            else
                provider.launcher.open = false;
        }
    }
}
