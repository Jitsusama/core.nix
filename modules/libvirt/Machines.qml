// The VMs in the launcher, as Programs › VMs: hidden while no VM exists
// on either connection, and once one does, each machine a row inside it,
// its name, "system" after it for one on the system's connection, and its
// state at the right, in words.json's words. Return steps in to what can
// be done to it in that state (Machines.js's acts), then every figure
// dominfo gives as a row Return copies; its drawer shows the same
// figures, the plate's eyebrow saying which connection it's on. The
// program's row says below it how many are in each state.
//
// The program is the module's own hidden desktop entry (home.nix), whose
// Return steps in rather than starting virt-manager's manager, which the
// machine keeps out of the launcher (nixos.nix).
//
// virsh is asked as the launcher opens, and again as Programs › VMs is
// stepped into for a connection nothing follows yet, never on a timer:
// asking the session's connection wakes the account's virtqemud through
// its socket, which quits two minutes after its last client, so it's
// asked only while Joel looks. While the launcher is open and a connection
// holds a machine, `virsh event` follows it, and each change it hears is
// read whole. The system's connection is read with -r, its read-only
// socket, which polkit never asks about.
//
// Starting, resuming, powering off and forcing off a session machine run
// virsh on its connection, the row showing what it becomes at once; one on
// the system's goes through polkit, so its acts say they'll ask for his
// password, run only on his own Return (Catalogue.js's disruptive), and
// close the launcher first, so the password prompt takes the keys. Force
// It Off ends what the machine was doing, so Return writes :forceoff on
// the command line and asks once before it runs, as every ending does
// (Words.ending). Open Its Console opens the machine's screen in
// virt-manager, through niri.
pragma ComponentBehavior: Bound
import qs
import Quickshell
import Quickshell.Io
import QtQuick
import "Machines.js" as Machines

Provider {
    id: provider

    what: Words.name("vm")

    // Its settings from home.nix: where virsh, virt-manager and stdbuf are,
    // and the desktop entry the program's row is made from.
    readonly property string virsh: settings.virsh ?? ""
    readonly property string virtManager: settings.virtManager ?? ""
    readonly property string stdbuf: settings.stdbuf ?? ""
    readonly property string program: settings.program ?? ""

    readonly property var said: Words.of("vm").said ?? {}
    readonly property var tells: Words.of("vm").tells ?? {}

    // Whether Joel can see what it says; only then is virsh asked.
    readonly property bool shown: launcher !== null && launcher.open
    // Whether Programs › VMs is held, or anything below it.
    readonly property bool steppedIn: shown && program !== ""
        && (launcher.held ?? []).some(step => step.row?.key === `program:${program}`)

    // Programs lists the program only while a machine exists, as virsh
    // last said: kept from one opening to the next, so the row doesn't
    // come and go under Joel's eyes as each opening reads it again.
    readonly property bool any: inSession.machines.length > 0 || inSystem.machines.length > 0
    hiddenPrograms: any && program !== "" ? [program] : []

    onShownChanged: {
        for (const reader of [inSession, inSystem]) {
            if (shown)
                reader.look();
            else
                reader.read = false;
        }
    }

    onSteppedInChanged: {
        if (!steppedIn)
            return;
        for (const reader of [inSession, inSystem]) {
            if (!reader.watching)
                reader.look();
        }
    }

    // What the rows show, so Programs › VMs, held, is drawn again as it
    // changes, and only then.
    readonly property string shows: JSON.stringify([inSession.read, inSession.machines,
        inSystem.read, inSystem.machines])
    onShowsChanged: changed()

    // Every machine, each marked with the connection it's on.
    function machines() {
        const marked = (reader) => reader.machines.map(one => Object.assign({
            system: reader.system
        }, one));
        return marked(inSession).concat(marked(inSystem));
    }

    function readerOf(one) {
        return one.system ? inSystem : inSession;
    }

    function keyOf(one) {
        return `vm:${readerOf(one).name}:${one.uuid}`;
    }

    // Below the program's row, how many are in each state, of those read
    // since the launcher opened, so it never says a state that was.
    function programNote(asked) {
        if (asked !== program)
            return "";
        return Machines.tally(machines().filter(one => readerOf(one).read), said);
    }

    function programActs(asked) {
        if (asked !== program)
            return [];
        return Machines.ordered(machines()).map(one => provider.machineRow(one));
    }

    function machineRow(one) {
        const state = Machines.stateOf(one.state);
        return thingRow({
            thing: true,
            key: keyOf(one),
            name: one.name,
            count: one.system ? said.system : "",
            side: readerOf(one).read ? Machines.side(state, said) : said.reading,
            glyph: Words.glyph("vm"),
            keywords: one.uuid,
            drawer: () => provider.drawerOf(one)
        }, () => provider.holds(one, state));
    }

    // The plate: which connection it's on, its name, every figure dominfo
    // gave, and on the system's, that changing it asks for his password.
    function drawerOf(one) {
        return {
            eyebrow: one.system ? said.inSystem : said.session,
            title: one.name,
            facts: Machines.figures(one, said).map(figure => ({
                label: Words.name(`vm:${figure.id}`),
                value: figure.value
            })),
            body: one.system ? tells.system ?? "" : ""
        };
    }

    // What a machine holds a step in: what can be done in its state, once
    // its connection has been read since the launcher opened, then each
    // figure, Return copying it.
    function holds(one, state) {
        const acts = readerOf(one).read
            ? Machines.acts(state).map(act => provider.actRow(one, act)) : [];
        return acts.concat(Machines.figures(one, said).map(figure => copyRow({
            key: `${keyOf(one)}:${figure.id}`,
            name: Words.name(`vm:${figure.id}`),
            side: figure.value,
            copy: figure.value,
            hue: "machine",
            glyph: ""
        })));
    }

    function actRow(one, act) {
        const id = `verb:vm:${act}`;
        const changes = act !== "console";
        return thing(Object.assign({
            key: `${id}:${readerOf(one).name}:${one.uuid}`,
            // Its words by id, as its key goes on past them.
            wordsId: id,
            name: Words.name(id),
            verb: Words.verb(id),
            glyph: Words.glyph(id),
            hue: "action",
            note: one.system ? said.password : "",
            // Only Joel's own Return does one on the system's, never a run
            // asked from outside, since polkit's prompt would come up on
            // his screen for something he didn't press.
            confirm: one.system,
            // One that changes a session machine keeps the launcher, the
            // row showing what it becomes; one on the system's closes it
            // first, so polkit's prompt has the keys.
            stays: changes && !one.system,
            later: changes && one.system,
            run: () => {
                if (changes)
                    provider.act(one, act);
                else
                    provider.openConsole(one);
            }
        }, Words.ending(id), act === "resume" ? {
            // Resuming is taken back by pausing it again.
            undo: () => provider.act(one, "pause")
        } : {}));
    }

    // Its screen, in virt-manager, which opens a window through niri.
    function openConsole(one) {
        Niri.spawn([virtManager, "--connect", readerOf(one).uri, "--show-domain-console",
            one.name]);
    }

    // The word Becoming.js says an act wasn't done by, as words.json's
    // field.becoming.did names them.
    readonly property var did: ({
        "start": "start",
        "resume": "resume",
        "pause": "pause",
        "power-off": "off",
        "force-off": "off"
    })

    // virsh told to do an act on the machine's connection, read-write, the
    // row drawn as what it becomes until virsh's next read agrees; powering
    // off is left to the guest, so it's done once the row says shut off.
    // A system machine waits on his password, so it's given two minutes.
    function act(one, doing) {
        const reader = readerOf(one);
        const key = keyOf(one);
        const next = Machines.becomes(doing);
        const off = Machines.side("off", said);
        becomes(key, next === "" ? {} : { side: Machines.side(next, said) }, one.name,
            did[doing] ?? doing, Object.assign({ within: one.system ? 120000 : 30000 },
                doing === "power-off" ? { done: row => row === null || row.side === off } : {}));
        if (one.system && launcher?.open)
            launcher.open = false;
        const asking = acting.createObject(provider) as Asked;
        asking.answered.connect((code, out, err) => {
            if (code !== 0)
                provider.refused(key, Machines.reason(err));
            reader.look();
            asking.destroy();
        });
        asking.ask(reader.acting.concat(Machines.told(doing, one.uuid)));
    }

    Component {
        id: acting

        Asked {}
    }

    Reader {
        id: inSession

        uri: "qemu:///session"
        system: false
        virsh: provider.virsh
        stdbuf: provider.stdbuf
        shown: provider.shown
    }

    Reader {
        id: inSystem

        uri: "qemu:///system"
        system: true
        virsh: provider.virsh
        stdbuf: provider.stdbuf
        shown: provider.shown
    }

    // virsh run once, in C's words: what it said and how it ended, once
    // it has ended and both its streams are read, whichever comes last.
    component Asked: Process {
        id: asked

        signal answered(int code, string out, string err)

        property int heard: 0
        property int code: -1

        function ask(said) {
            heard = 0;
            code = -1;
            command = said;
            running = true;
        }

        function ended() {
            heard += 1;
            if (heard === 3)
                answered(code, out.text, err.text);
        }

        // Quickshell's type files say a hash where it takes any object, as
        // its own documentation gives one, so qmllint can't take this.
        environment: ({ LC_ALL: "C" }) // qmllint disable incompatible-type
        stdout: StdioCollector {
            id: out

            onStreamFinished: asked.ended()
        }
        stderr: StdioCollector {
            id: err

            onStreamFinished: asked.ended()
        }
        onExited: exitCode => {
            asked.code = exitCode;
            asked.ended();
        }
    }

    // One connection's machines: listed by UUID, then each described in one
    // batch, and followed while anyone looks and one exists.
    component Reader: Scope {
        id: reader

        required property string uri
        required property bool system
        required property string virsh
        required property string stdbuf
        required property bool shown

        readonly property string name: system ? "system" : "session"
        // virsh reading this connection: the system's read-only, where a
        // connection that could change anything would ask polkit.
        readonly property var reading: [virsh].concat(system ? ["-r"] : [], ["-c", uri])
        // virsh acting on it, which on the system's asks polkit.
        readonly property var acting: [virsh, "-c", uri]

        // What virsh last said, kept from one opening to the next, and
        // whether it's been said since the launcher opened.
        property var machines: []
        property bool read: false
        property bool asking: false
        // Asked again while being read, as a change heard mid-read is.
        property bool again: false
        readonly property bool watching: watch.running

        function look() {
            if (virsh === "")
                return;
            if (asking) {
                again = true;
                return;
            }
            asking = true;
            lister.ask(reading.concat(["list", "--all", "--uuid"]));
        }

        function listed(code, out) {
            const uuids = Machines.listed(out);
            if (code !== 0 || uuids.length === 0) {
                found([]);
                return;
            }
            describer.ask(reading.concat([uuids.map(uuid => `dominfo ${uuid}`).join("; ")]));
        }

        // What it found, read for this opening only if it's still open.
        function found(told) {
            machines = told;
            read = shown;
            asking = false;
            if (again) {
                again = false;
                look();
            }
        }

        Asked {
            id: lister

            onAnswered: (code, out) => reader.listed(code, out)
        }

        Asked {
            id: describer

            onAnswered: (code, out) => reader.found(Machines.described(out))
        }

        // Each machine's starts and stops, as virsh hears them, for as long
        // as anyone looks and a machine is there to follow. Without
        // --domain it hears every machine, and --all can't be given with
        // --event. stdbuf has virsh write each line as it says it, which
        // it would otherwise hold until its buffer filled.
        Process {
            id: watch

            running: reader.shown && reader.machines.length > 0 && reader.virsh !== ""
                && reader.stdbuf !== ""
            command: [reader.stdbuf, "-oL"].concat(reader.reading,
                ["event", "--loop", "--event", "lifecycle"])
            // As Asked's, which says why qmllint can't take it.
            environment: ({ LC_ALL: "C" }) // qmllint disable incompatible-type
            stdout: SplitParser {
                onRead: line => {
                    if (Machines.changed(line))
                        reader.look();
                }
            }
        }
    }
}
