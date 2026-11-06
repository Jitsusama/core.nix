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
                    readonly property string asker:
                        `<span style="font-family:'${Theme.mono.family}'">${root.asker}</span>`
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
