// The prompt programs ask through for a PIN, a yes or a touch: SSH's askpass
// (ssh-askpass, which home.nix makes and SSH_ASKPASS names). ssh-tpm-agent
// runs it before a key confirmed on use signs and for a key's PIN, and ssh
// for a security key's. It's polkit's prompt in all but its words, which
// Askpass.js reads from the program's: a scrim, the accent frame as it holds
// the keys, who's asking for what, the padlock beside the dots when a secret
// is asked for, and what the keys do. Return answers and Escape refuses. A
// touch is said with no scrim and no keys, since nothing typed answers it,
// and goes once the key's touched, when ssh stops the askpass.
//
// The askpass makes a folder only Joel can open in the runtime folder, and
// asks through IPC, naming it. The answer is written there as "answer", or
// "refused", and the askpass says it and deletes the folder, so a PIN rests
// only in memory, and only for that moment. A second program asking waits
// until the first is answered. Asks live only here, so an askpass checks this
// still has its own while it waits, and asks again after a restart.
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import QtQuick
import "Askpass.js" as Asked

Scope {
    id: root

    // As wide as polkit's prompt.
    readonly property int width: 560
    // The large step's line, to a whole pixel on the space grid.
    readonly property int line: Theme.space.xl + Theme.space.xs / 2
    readonly property string runtime: Quickshell.env("XDG_RUNTIME_DIR")

    // Who's waiting, oldest first: { kind, asker, prompt, reply }.
    property var asks: []
    readonly property var asking: asks.length > 0 ? asks[0] : null
    readonly property var read: asking
        ? Asked.read(asking.kind, asking.asker, asking.prompt, Quickshell.env("HOME")) : null
    // Whether it takes the keys, which a touch doesn't.
    readonly property bool holds: read !== null && read.keys.length > 0

    function escaped(text) {
        return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
    }

    // Answers the oldest ask, in its folder, and asks the next.
    function settle(name, text) {
        const ask = asking;
        if (!ask)
            return;
        const file = writer.createObject(root, { path: `${ask.reply}/${name}` }) as FileView;
        file.setText(text);
        file.destroy();
        asks = asks.filter(other => other !== ask);
    }

    Component {
        id: writer

        FileView {
            printErrors: false
            blockWrites: true
        }
    }

    IpcHandler {
        target: "askpass"

        // Asks what the program's prompt says, for the askpass waiting on the
        // folder reply. Only a folder an askpass makes is written to.
        function ask(kind: string, asker: string, prompt: string, reply: string): void {
            const folder = reply.startsWith(`${root.runtime}/`)
                ? reply.slice(root.runtime.length + 1) : "";
            if (/^askpass\.[A-Za-z0-9]+$/.test(folder) && !waiting(reply))
                root.asks = root.asks.concat([{ kind, asker, prompt, reply }]);
        }

        // Whether the askpass waiting on the folder reply is still being asked
        // for, which a shell that restarted since has forgotten.
        function waiting(reply: string): bool {
            return root.asks.some(ask => ask.reply === reply);
        }

        // Stops asking, once the program has its answer some other way or
        // gave up waiting.
        function withdraw(reply: string): void {
            root.asks = root.asks.filter(ask => ask.reply !== reply);
        }
    }

    // Each ask starts with nothing typed and with the keys where they answer.
    onReadChanged: {
        prompt.clear();
        if (read !== null && read.field)
            prompt.take();
        else
            yes.forceActiveFocus();
        reported.changed();
    }

    PanelWindow {
        Counted { name: "askpass" }

        visible: root.asking !== null
        anchors.top: true
        anchors.bottom: true
        anchors.left: true
        anchors.right: true
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        // A touch leaves the screen to be used while it waits.
        mask: root.holds ? null : frame.area
        BackgroundEffect.blurRegion: Region {
            item: root.holds ? scrim : null
        }
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: root.holds ? WlrKeyboardFocus.Exclusive
            : WlrKeyboardFocus.None
        WlrLayershell.namespace: "askpass"

        Reported {
            id: reported
            name: "askpass"
        }

        Rectangle {
            id: scrim
            anchors.fill: parent
            visible: root.holds
            color: Qt.alpha(Theme.deepest, Theme.floating.scrim)
        }

        // Where the keys go when there's nothing to type: Return allows and
        // Escape refuses.
        Item {
            id: yes
            focus: true
            Keys.onReturnPressed: root.settle("answer", "yes\n")
            Keys.onEnterPressed: root.settle("answer", "yes\n")
            Keys.onEscapePressed: root.settle("refused", "\n")
        }

        Floating {
            id: frame
            anchors.centerIn: parent
            width: root.width
            keys: root.holds

            Column {
                width: parent.width

                Column {
                    x: Theme.inset + Theme.space.l
                    width: parent.width - 2 * x
                    // As polkit's: the letters as far from the frame as from
                    // its sides, measured to the ink.
                    topPadding: Theme.inset + Theme.space.l - Theme.border - 5
                    bottomPadding: Theme.inset + Theme.space.l - 2

                    Text {
                        width: parent.width
                        textFormat: Text.RichText
                        // As polkit's: programs in the monospace face, the
                        // thing asked for strong.
                        readonly property var styles: ({
                            program: `font-family:'${Theme.mono.family}'`,
                            thing: `color:${Theme.strong};font-weight:500`
                        })
                        text: root.read === null ? "" : root.read.title.map(part => {
                            const words = root.escaped(part.text);
                            return part.as in styles
                                ? `<span style="${styles[part.as]}">${words}</span>` : words;
                        }).join("")
                        color: Theme.text
                        wrapMode: Text.Wrap
                        lineHeight: root.line
                        lineHeightMode: Text.FixedHeight
                        font.family: Theme.sans.family
                        font.pointSize: Theme.sans.large
                    }

                    Text {
                        width: parent.width
                        visible: text !== ""
                        text: root.read === null ? "" : root.read.detail
                        color: Theme.muted
                        wrapMode: Text.Wrap
                        lineHeight: root.line
                        lineHeightMode: Text.FixedHeight
                        font.family: root.read !== null && root.read.mono ? Theme.mono.family
                            : Theme.sans.family
                        font.pointSize: root.read !== null && root.read.mono ? Theme.mono.large
                            : Theme.sans.large
                    }
                }

                Rectangle {
                    width: parent.width
                    height: Theme.border
                    visible: field.visible
                    color: Theme.edge
                }

                Item {
                    id: field
                    width: parent.width
                    height: prompt.height + 2 * Theme.inset
                    visible: root.read !== null && root.read.field

                    Prompt {
                        id: prompt
                        x: Theme.inset
                        y: Theme.inset
                        width: parent.width - 2 * Theme.inset
                        glyph: Theme.glyph.secret
                        hue: Theme.machine
                        secret: root.read === null || !root.read.shows
                        onAccepted: root.settle("answer", `${text}\n`)
                        onCancelled: root.settle("refused", "\n")
                    }
                }

                Rectangle {
                    width: parent.width
                    height: Theme.border
                    visible: root.holds
                    color: Theme.edge
                }

                Item {
                    width: parent.width
                    height: Theme.space.xxl + Theme.space.s
                    visible: root.holds

                    KeyHints {
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.space.l
                        anchors.verticalCenter: parent.verticalCenter
                        keys: root.read === null ? [] : root.read.keys
                    }
                }
            }
        }
    }
}
