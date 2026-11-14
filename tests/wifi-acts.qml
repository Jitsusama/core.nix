// Drives the launcher's Wi-Fi (launcher/Wireless.qml) through Quickshell
// itself, with the real ReadyRows.qml making each network's row and acts
// from the ones wifid serves (modules/wifid/daemon/src/rows.rs), the
// launcher stood in as far as it asks for words, and Wifi's card stood in.
// Records are told through ReadyRows' own Daemon.qml's took, and what's
// sent goes to a stand-in wire, answered by its hear, so every case runs
// in one turn with no daemon. A network met for the first time asks its
// password in the field and is sent it set; one asking for an account asks
// its name, its server's domain and its password in turn, each set at its
// place in the ask, and the ask is sent after the last; a step out partway
// sends nothing and says so; a join's stages and a refusal are said in the
// foot, never a card; and a secret is never shown. It exits 0 once every
// case holds, and otherwise 1, saying which didn't.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher
import "Expect.js" as Expect

ShellRoot {
    id: root

    // A network's row as wifid serves it, at `order`, its acts `acts`.
    function network(name, security, order, acts) {
        const key = `row:wifi:network:${name}`;
        return { key, kind: "row", row: {
            key, thing: `wifi:${name}`, place: "wifi", kind: "network", name,
            note: security === "enterprise" ? "an account" : "a password",
            glyph: "lockedSignal3", hue: "machine", side: [{ said: "3 of 4 bars" }],
            held: { network: name, security, strength: 3, connected: false, known: false },
            acts, order
        } };
    }

    function joining(name, account, fields) {
        const ask = { ask: "act", act: "join", network: name };
        if (account)
            ask.account = { method: account };
        return { key: "verb:wifi-join", ask, streams: true, fields };
    }

    // A field as wifid serves it, a password's secret.
    function field(asks, into) {
        return Object.assign({ ask: `wifi:asks:${asks}`, into },
            asks === "password" ? { secret: true } : {});
    }

    readonly property var account: [field("identity", "account.identity"),
        field("server", "account.serverDomain"), field("password", "password")]

    // What the launcher does with a row asking for words: holds it as a step, its line typed
    // into, and hands what's typed to the step held last (Launcher.qml's ask, give and back).
    QtObject {
        id: standIn

        property bool open: true
        property var held: []
        readonly property var asking: held.length > 0 && held[held.length - 1].row.given
            ? held[held.length - 1].row : null
        property var selected: null
        property var terminalSees: ({ walked: [] })
        property var sources: [wireless]
        function wants(source) {
            return open;
        }
        function ask(row) {
            held = held.concat([{ row: row }]);
        }
        function give(words) {
            asking.given(words);
        }
        function back() {
            held = held.slice(0, -1);
        }
        function becomesAll(provider, holds, how) {
        }
        function becameNot(key, why) {
            Expect.fail(`a card was told for ${key}: ${why}`);
        }
    }

    Launcher.Wireless {
        id: wireless

        launcher: standIn
        settings: ({ name: "wifi" })
    }

    // What the shell sends wifid, each line as it's written, and the
    // joins among them.
    property var sent: []
    readonly property var wire: ({
            write: line => root.sent.push(JSON.parse(line)),
            flush: () => {}
        })
    function joins() {
        return sent.filter(said => said.act === "join");
    }

    function rowOf(name) {
        return wireless.all.find(made => made.network === name) ?? null;
    }

    // What the cursor's row says in the foot now.
    function foot(row) {
        return typeof row.state === "function" ? row.state() : row.state;
    }

    // The cases run once ReadyRows' daemon is loaded, which its LazyLoader may do after the
    // shell starts.
    property bool begun: false
    Component.onCompleted: begin()
    Connections {
        target: wireless.served
        function onLinkChanged() {
            root.begin();
        }
    }

    function begin() {
        if (begun || !wireless.served.link)
            return;
        begun = true;
        try {
            wireless.served.link.wire = root.wire;
            wireless.served.link.took([
                network("Lantern", "personal", 1024,
                    [joining("Lantern", null, [field("password", "password")])]),
                network("Campus", "enterprise", 2048, [joining("Campus", "peap", account)])
            ], []);
            password();
            accounts();
            partway();
            Qt.exit(0);
        } catch (error) {
            Expect.fail(`${error} at ${error.stack}`);
        }
    }

    // A network met for the first time asks its password in the field, the padlock's, and is
    // sent it, its steps said in the foot as wifid words them until it's done, the field put
    // away; a password refused leaves the field open to try again, saying why.
    function password() {
        const lantern = rowOf("Lantern");
        Expect.same("a network met behind a password asks it at Return, typed as a secret",
            [lantern.ask, lantern.secret, lantern.chip], ["password", true, "Lantern"]);
        standIn.ask(lantern);
        standIn.give("correct horse");
        const asked = joins();
        Expect.expect("its join is sent with the password set",
            asked.length === 1 && typeof asked[0].password === "string"
            && asked[0].network === "Lantern", JSON.stringify(asked.map(one => one.network)));
        const id = asked[0].id;
        wireless.served.link.hear(JSON.stringify({ id, error: { code: "refused",
            refused: "WrongPassword", said: "that password didn't work" } }));
        Expect.same("a password refused says so in the foot, the field still open",
            [foot(lantern), standIn.held.length], ["that password didn't work", 1]);
        standIn.give("battery staple");
        const again = joins()[1].id;
        wireless.served.link.hear(JSON.stringify({ id: again, stage: "joining",
            said: "connecting to Lantern\u2026", done: false }));
        Expect.same("each step is said in the foot as wifid words it",
            foot(lantern), "connecting to Lantern\u2026");
        Expect.expect("and the field waits meanwhile", standIn.held.length === 1, "");
        wireless.served.link.hear(JSON.stringify({ id: again, stage: "online",
            said: "connected to Lantern", done: true }));
        Expect.same("done, the field is put away, the foot saying how it ended",
            [standIn.held.length, foot(lantern)], [0, "connected to Lantern"]);
        const shown = JSON.stringify(wireless.served.saying);
        Expect.expect("no password is ever shown in the foot",
            !shown.includes("horse") && !shown.includes("staple"), "");
    }

    // One asking for an account asks its name, its server's domain and its password in turn,
    // each in the one field, the answer before it beside it but for a secret, and is sent once
    // the last is given, each answer set at its place in the ask.
    function accounts() {
        const campus = rowOf("Campus");
        const before = joins().length;
        standIn.ask(campus);
        const asked = [standIn.asking.ask];
        standIn.give("ada");
        asked.push(standIn.asking.ask);
        const server = standIn.asking;
        standIn.give("campus.example.org");
        asked.push(standIn.asking.ask);
        const secret = standIn.asking;
        Expect.same("an account asks its name, its server's domain, then its password",
            asked, ["the account's name", "its server's domain", "password"]);
        Expect.same("each in the one field, in the place of the one before",
            standIn.held.length, 1);
        Expect.same("each beside the answer before it, the password alone a secret",
            [server.chip, server.secret, secret.chip, secret.secret],
            ["ada", false, "campus.example.org", true]);
        Expect.same("nothing's sent before the last", joins().length, before);
        standIn.give("analytical engine");
        const join = joins()[before];
        Expect.same("then it's sent, each answer at its place, the method served kept",
            join.account, { method: "peap", identity: "ada",
                serverDomain: "campus.example.org" });
        Expect.expect("and its password set", typeof join.password === "string", "");
        wireless.served.link.hear(JSON.stringify({ id: join.id, stage: "online",
            said: "connected to Campus", done: true }));
        Expect.same("connected, the field is put away", standIn.held.length, 0);
    }

    // A step out of an account's fields before the last sends nothing, every answer let go,
    // and the foot says so.
    function partway() {
        const campus = rowOf("Campus");
        const before = joins().length;
        standIn.ask(campus);
        standIn.give("ada");
        standIn.back();
        Expect.same("a step out partway sends nothing", joins().length, before);
        Expect.same("and the foot says what was typed was let go", foot(campus),
            Words.field.becoming.dropped);
        standIn.ask(campus);
        Expect.same("asked again, it starts from its first", standIn.asking.ask,
            "the account's name");
        Expect.expect("the foot saying nothing of the last", foot(campus) !== Words.field
            .becoming.dropped, foot(campus));
    }
}
