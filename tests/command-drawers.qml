// Every command the launcher offers says a little more in its drawer than
// its line (docs/design.md, Every row says a little more beside it), run in
// Qt's own JavaScript engine, the one the shell runs it in. The
// `every-command-row-has-a-drawer` check copies
// modules/quickshell/launcher/CommandRow.js and the vocabulary beside this
// file and runs it offscreen. Each command's drawer says what it answers in
// a sentence its line doesn't, then one or two worked examples, each typed
// as the command starts, and an address in one is a documentation address,
// so no owner's answer can go stale (Y2YBTS, f7ea4559).
import QtQuick
import "CommandRow.js" as CommandRow
import "Expect.js" as Expect

QtObject {
    // A test that throws ends as a failure, rather than leaving qml running.
    Component.onCompleted: {
        try {
            run();
        } catch (error) {
            Expect.fail(error.message);
        }
    }

    // The addresses set aside for documentation, which a worked example
    // may name: 192.0.2.0/24, 198.51.100.0/24 and 203.0.113.0/24 (RFC
    // 5737) and 2001:db8::/32 (RFC 3849).
    readonly property var documented: [/^192\.0\.2\./u, /^198\.51\.100\./u, /^203\.0\.113\./u]

    // The vocabulary as the shell reads it, which the check writes beside
    // this file.
    function vocabulary() {
        const request = new XMLHttpRequest();
        request.open("GET", Qt.resolvedUrl("words.json"), false);
        request.send();
        return JSON.parse(request.responseText);
    }

    // Every address of four numbers an example says.
    function addresses(text) {
        return String(text).match(/\d+\.\d+\.\d+\.\d+/gu) ?? [];
    }

    // Every IPv6 address an example says, as hex groups around the double
    // colon one shortened by it holds.
    function sixes(text) {
        return (String(text).match(/[0-9a-f:]*::[0-9a-f:]*/gu) ?? [])
            .filter(address => /[0-9a-f]/u.test(address));
    }

    function held(command, words) {
        const what = command.id;
        // Its eyebrow says where it's listed, then what it is, as every
        // drawer's does: "in Network · command", or "anywhere · command".
        const eyebrow = CommandRow.eyebrow(command, words.categories,
            words.field.drawer.command);
        const place = words.categories.find(category => (category.commands ?? [])
            .includes(what));
        Expect.same(`${what}'s eyebrow says where it's listed`, eyebrow,
            place ? `in ${place.name} · command` : "anywhere · command");
        const said = CommandRow.drawer(command, eyebrow);
        Expect.expect(`${what} has a drawer`, said !== null, "it gave none");
        Expect.same(`${what}'s drawer is under its eyebrow`, said.eyebrow, eyebrow);
        Expect.same(`${what}'s drawer is titled as its row is`, said.title,
            `${command.start}${command.form}`.trim());
        Expect.expect(`${what}'s drawer says what it answers in a sentence`,
            /^[A-Z].*\.$/u.test(said.body), `it said "${said.body}"`);
        Expect.expect(`${what}'s sentence says more than its line`, said.body !== command.says,
            `it said only "${said.body}"`);
        const count = said.facts.length;
        Expect.expect(`${what} gives one or two worked examples`, count >= 1 && count <= 2,
            `it gave ${count}`);
        for (const fact of said.facts) {
            Expect.expect(`${what}'s example is typed as the command starts`,
                String(fact.label).startsWith(command.start)
                    && (fact.label !== command.start || command.form === ""),
                `it reads "${fact.label}"`);
            Expect.expect(`${what}'s example says what comes back`, String(fact.value) !== "",
                `"${fact.label}" says nothing`);
            const said4 = [...addresses(fact.label), ...addresses(fact.value)];
            for (const address of said4) {
                Expect.expect(`${what}'s example names a documentation address`,
                    documented.some(range => range.test(address)), `it names ${address}`);
            }
            const said6 = [...sixes(fact.label), ...sixes(fact.value)];
            for (const address of said6) {
                Expect.expect(`${what}'s example names a documentation address`,
                    address.startsWith("2001:db8:"), `it names ${address}`);
            }
        }
    }

    function run() {
        const words = vocabulary();
        Expect.expect("the vocabulary offers commands", words.commands.length > 0,
            "it offers none");
        for (const command of words.commands)
            held(command, words);
        Expect.same("a command Network lists is said in Network",
            CommandRow.eyebrow(words.commands.find(command => command.id === "command:ip"),
                words.categories, words.field.drawer.command), "in Network · command");
        Expect.same("a command no place lists is said anywhere",
            CommandRow.eyebrow(words.commands.find(command => command.id === "command:sum"),
                words.categories, words.field.drawer.command), "anywhere · command");

        // One the vocabulary says nothing more of has none, so its row
        // draws no drawer that repeats it.
        Expect.same("a command with nothing more to say has no drawer",
            CommandRow.drawer({ id: "bare", start: "a ", form: "b", says: "c" }, "command"),
            null);
        Expect.expect("a documentation address is told from another",
            !documented.some(range => range.test("8.8.8.8")), "8.8.8.8 passed");

        // biome-ignore lint/suspicious/noConsole: a QML test's result is said on its log.
        console.log(`command-drawers: all ${words.commands.length} commands say more`);
        Qt.exit(0);
    }
}
