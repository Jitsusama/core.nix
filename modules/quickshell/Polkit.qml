// The polkit agent: when a program asks to do something as root, a scrim
// stops the screen and a prompt asks for Joel's password. It's the prompt
// kind (docs/design.md, Shape and Kinds): dead centre, the accent frame as
// it holds the keys, who's asking and why in words, then the padlock beside
// the dots, then what the keys do. A wrong password turns the frame to the
// alert and says so. Escape refuses.
//
// polkit says which action is asked for but not which program asked, so the
// asker is the one that owns the action: org.freedesktop.systemd1's actions
// are systemd's.
pragma ComponentBehavior: Bound
import Quickshell
import Quickshell.Services.Polkit
import Quickshell.Wayland
import QtQuick

Scope {
    id: root

    // Wide enough for polkit's longer reasons on two lines at most.
    readonly property int width: 560
    // The large step's line, to a whole pixel on the space grid.
    readonly property int line: Theme.space.xl + Theme.space.xs / 2

    PolkitAgent {
        id: agent
    }

    readonly property var flow: agent.flow
    readonly property bool failed: flow !== null && flow.supplementaryIsError
        && flow.supplementaryMessage !== ""
    // The program owning the action: its name after its maker's reversed
    // domain, without the version a D-Bus name ends in.
    readonly property string asker: {
        const parts = flow ? flow.actionId.split(".") : [];
        const known = { policykit: "pkexec", login: "logind" };
        const name = parts.length > 3 ? parts[2].replace(/\d+$/, "") : "";
        return known[name] ?? (name || "polkit");
    }
    readonly property string who: {
        const identity = flow ? flow.selectedIdentity : null;
        return identity ? (identity.displayName || identity.name || "") : Quickshell.env("USER");
    }

    // Polkit's message says what it's for as "Authentication is required to
    // do this"; the prompt says "To do this."
    readonly property string why: {
        if (!flow)
            return "";
        const asked = /^authentication is (required|needed) (in order )?to /i;
        const message = flow.message.trim().replace(asked, "");
        const sentence = message.charAt(0).toUpperCase() + message.slice(1);
        const reason = message === flow.message.trim() ? sentence : "To " + message;
        return /[.!?]$/.test(reason) ? reason : reason + ".";
    }

    PanelWindow {
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

                Column {
                    x: Theme.inset + Theme.space.l
                    width: parent.width - 2 * x
                    // The letters sit as far from the frame as from its
                    // sides. A line is taller than its letters, so each
                    // side gives back what the lines add beyond them,
                    // measured to the ink on the panel.
                    topPadding: Theme.inset + Theme.space.l - Theme.border - 5
                    bottomPadding: Theme.inset + Theme.space.l - 2

                    Text {
                        width: parent.width
                        textFormat: Text.RichText
                        readonly property string asker:
                            `<span style="font-family:'${Theme.mono.family}'">${root.asker}</span>`
                        readonly property string owner:
                            `<span style="color:${Theme.strong};font-weight:500">${root.who}</span>`
                        text: `${asker} needs ${owner}'s password`
                        color: Theme.text
                        wrapMode: Text.Wrap
                        lineHeight: root.line
                        lineHeightMode: Text.FixedHeight
                        font.family: Theme.sans.family
                        font.pointSize: Theme.sans.large
                    }

                    Text {
                        width: parent.width
                        text: root.failed ? root.flow.supplementaryMessage : root.why
                        color: root.failed ? Theme.alert : Theme.muted
                        wrapMode: Text.Wrap
                        lineHeight: root.line
                        lineHeightMode: Text.FixedHeight
                        font.family: Theme.sans.family
                        font.pointSize: Theme.sans.large
                    }
                }

                Rectangle {
                    width: parent.width
                    height: Theme.border
                    color: Theme.edge
                }

                Item {
                    width: parent.width
                    height: prompt.height + 2 * Theme.inset

                    Prompt {
                        id: prompt
                        x: Theme.inset
                        y: Theme.inset
                        width: parent.width - 2 * Theme.inset
                        glyph: String.fromCodePoint(0xf023)
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

                Rectangle {
                    width: parent.width
                    height: Theme.border
                    color: Theme.edge
                }

                Item {
                    width: parent.width
                    height: Theme.space.xxl + Theme.space.s

                    KeyHints {
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.space.l
                        anchors.verticalCenter: parent.verticalCenter
                        keys: [["\u21b5", "allow"], ["Esc", "refuse"]]
                    }
                }
            }
        }
    }
}
