// The polkit agent: when a program asks to do something as root, a scrim
// stops the screen and a prompt asks for Joel's password. It's the prompt
// kind (docs/design.md, Shape and Kinds): dead centre, the accent frame as
// it holds the keys, who's asking and why in words, then the padlock beside
// the dots, then what the keys do. A wrong password turns the frame to the
// alert and says so. Escape refuses.
//
// Who asks is the command that called polkit, read from /proc by the pid
// polkit gives with each ask, so an agent's run0 shows the command it would
// run as root, never only "systemd" (Polkit.js says each). One gone before
// it's read is named by the program owning the action.
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

        Rectangle {
            id: scrim
            anchors.fill: parent
            color: Qt.alpha(Theme.deepest, Theme.floating.scrim)
        }

        Floating {
            anchors.centerIn: parent
            width: root.width
            keys: true
            urgent: root.failed

            Column {
                width: parent.width

                // Who asks in the mono, as a program is named, and whose
                // password a strong span; then why, or what went wrong.
                PromptHead {
                    width: parent.width
                    readonly property string asker: `<span style="font-family:`
                        + `'${Theme.mono.family}'">${Said.plain(root.asker)}</span>`
                    readonly property string owner: `<span style="color:${Theme.strong};`
                        + `font-weight:${Theme.level.words.spans.strong}">${root.who}</span>`
                    asked: `${asker} needs ${owner}'s password`
                    askedOpens: root.asker.charAt(0)
                    askedOpensIn: Theme.mono.family
                    reason: root.failed ? root.flow.supplementaryMessage : root.why
                    reasonColour: root.failed ? Theme.alert : Theme.level.words.colour
                }

                Hairline {
                    width: parent.width
                }

                // The field, one row, its mark on the head's column.
                Item {
                    width: parent.width
                    height: prompt.height

                    Prompt {
                        id: prompt
                        x: Theme.space.rung.inset - markAt
                        width: parent.width - 2 * x
                        glyph: Theme.glyph.secret
                        hue: Theme.machine
                        secret: !(root.flow && root.flow.responseVisible)
                        onAccepted: {
                            root.flow.submit(text);
                            clear();
                        }
                        onCancelled: {
                            root.flow.cancelAuthenticationRequest();
                            clear();
                        }
                    }
                }

                Hairline {
                    width: parent.width
                }

                PromptFoot {
                    width: parent.width
                    keys: [["\u21b5", "allow"], ["Esc", "refuse"]]
                }
            }
        }
    }
}
