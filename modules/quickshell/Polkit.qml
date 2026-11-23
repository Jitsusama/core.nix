// The polkit agent: when a program asks to do something as root, a scrim
// stops the screen and a prompt asks for Joel's password. It's the prompt
// kind (docs/design.md, Shape and Kinds): dead centre, the accent frame as
// it holds the keys, who's asking and why in words, then the padlock beside
// the dots, then what the keys do. A wrong password turns the frame to the
// alert and says so. Escape refuses. The journal says as each ask begins
// and how it ended, after how long, and whether niri gave it the keys.
//
// Who asks is the command that called polkit, read from /proc by the pid
// polkit gives with each ask, so an agent's run0 shows the command it would
// run as root, never only "systemd" (Polkit.js says each). One gone before
// it's read is named by the program owning the action. Under the command,
// the quest the asking process says it runs under, as asker-says reads it,
// so Joel sees which agent asks; the shell never reads the process's
// environment itself, which holds secrets beside it (asker-says/src/main.rs).
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Io
import Quickshell.Services.Polkit
import Quickshell.Wayland
import QtQuick
import "Polkit.js" as Said

Scope {
    id: root

    // Wide enough for polkit's longer reasons on two lines at most.
    readonly property int width: 560

    PolkitAgent {
        id: agent
    }

    readonly property var flow: agent.flow
    readonly property bool failed: flow !== null && flow.supplementaryIsError
        && flow.supplementaryMessage !== ""
    // The command line of the process asking, its words ended by NULs, read
    // before the prompt's first frame; empty while nothing asks or once
    // it's gone.
    FileView {
        id: asking
        readonly property string pid: root.flow ? Said.askerPid(root.flow.details) : ""
        path: pid === "" ? "" : `/proc/${pid}/cmdline`
        printErrors: false
        blockLoading: true
    }
    readonly property string cmdline: asking.path === "" ? "" : asking.text()

    // What the asking process says of itself, asker-says's answer, asked
    // once per ask; empty until it answers. The card words it, as only it
    // can measure the line.
    property string saidOfItself: ""
    Process {
        id: sayer
        readonly property string reader: Quickshell.env("POLKIT_ASKER_SAYS") ?? ""
        command: [reader, "--json", asking.pid]
        running: reader !== "" && asking.pid !== ""
        onRunningChanged: {
            if (running)
                root.saidOfItself = "";
        }
        stdout: StdioCollector {
            onStreamFinished: root.saidOfItself = text
        }
    }

    readonly property string asker: flow ? Said.asker(cmdline, flow.actionId) : "polkit"
    onAskerChanged: {
        if (flow)
            Report.say(`polkit asked by ${asker}`);
    }
    readonly property string who: {
        const identity = flow ? flow.selectedIdentity : null;
        return identity ? (identity.displayName || identity.name || "") : Quickshell.env("USER");
    }

    readonly property string why: flow ? Said.reason(flow.message, cmdline, flow.details) : ""

    // The ask the journal follows, from polkit handing it over to its end,
    // reports on or not, so a failed ask is read from the record rather than
    // guessed (Polkit.js, begun, heard and finished). askSerial counts asks,
    // so an end said later belongs to its own ask and never to the next.
    // askFlow is the ask's own flow, read as it ends in case the agent moved
    // on before the flow's end signals reached the Connections below.
    property var ask: null
    property int askSerial: 0
    property var askFlow: null

    function askHeard(what) {
        if (ask)
            ask = Said.heard(ask, what, Date.now());
    }

    function askEnds(serial) {
        if (!ask || serial !== askSerial)
            return;
        if (askFlow?.isSuccessful)
            askHeard("answered");
        else if (askFlow?.isCancelled)
            askHeard("withdrawn");
        console.info(Said.finished(ask, Date.now()));
        ask = null;
        askFlow = null;
    }

    onFlowChanged: {
        askEnds(askSerial);
        if (!flow)
            return;
        askSerial += 1;
        askFlow = flow;
        ask = Said.begun(flow.actionId, Date.now(), keysHeld.keys);
        console.info(Said.began(ask.action));
    }

    Connections {
        target: root.flow

        function onIsSuccessfulChanged() {
            if (root.flow.isSuccessful)
                root.askHeard("answered");
        }
        function onIsCancelledChanged() {
            if (root.flow.isCancelled)
                root.askHeard("withdrawn");
        }
        function onAuthenticationFailed() {
            root.askHeard("wrong");
        }
        // Later, so how it ended is heard first, whichever order the flow
        // says its end in, and only for the ask that ended.
        function onIsCompletedChanged() {
            if (!root.flow.isCompleted)
                return;
            const serial = root.askSerial;
            Qt.callLater(() => root.askEnds(serial));
        }
    }

    PanelWindow {
        Counted { name: "polkit" }

        visible: agent.isActive
        anchors.top: true
        anchors.bottom: true
        anchors.left: true
        anchors.right: true
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        BackgroundEffect.blurRegion: Region {
            item: scrim
        }
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
        WlrLayershell.namespace: "polkit"

        Reported {
            name: "polkit"
        }

        // Whether niri gives the prompt the keys, for the journal's line on
        // the ask: held as an ask comes, given while it waits, or taken by
        // another surface, such as the lock, before it ends. Keys going as
        // the agent puts the prompt away are the ask ending, not a loss.
        Item {
            id: keysHeld
            readonly property bool keys: Window.active
            onKeysChanged: {
                if (keys)
                    root.askHeard("keys");
                else if (agent.isActive)
                    root.askHeard("keys-gone");
            }
        }

        Rectangle {
            id: scrim
            anchors.fill: parent
            color: Qt.alpha(Theme.deepest, Theme.floating.scrim)
        }

        PolkitCard {
            anchors.centerIn: parent
            width: root.width
            asker: root.asker
            who: root.who
            said: root.failed ? "" : root.saidOfItself
            reason: root.failed ? root.flow.supplementaryMessage : root.why
            failed: root.failed
            secret: !(root.flow && root.flow.responseVisible)
            // A password counts as sent only while the flow waits for one,
            // so a Return during a key's touch isn't taken for a try.
            onSubmitted: text => {
                if (root.flow.isResponseRequired)
                    root.askHeard("tried");
                root.flow.submit(text);
            }
            onRefused: {
                root.askHeard("refused");
                root.flow.cancelAuthenticationRequest();
            }
        }
    }
}
