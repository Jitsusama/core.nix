// The polkit agent: when a program asks to do something as root, the screen
// dims and a prompt asks for Joel's password, in the theme. Escape refuses.
import Quickshell
import Quickshell.Services.Polkit
import Quickshell.Wayland
import QtQuick

Scope {
    PolkitAgent {
        id: agent
    }

    PanelWindow {
        visible: agent.isActive
        anchors.top: true
        anchors.bottom: true
        anchors.left: true
        anchors.right: true
        exclusionMode: ExclusionMode.Ignore
        color: Qt.alpha(Theme.deepest, 0.8)
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
        WlrLayershell.namespace: "polkit"

        Column {
            anchors.centerIn: parent
            width: prompt.implicitWidth
            spacing: Theme.gap

            Text {
                width: parent.width
                text: agent.flow ? agent.flow.message : ""
                color: Theme.strong
                wrapMode: Text.Wrap
                font.family: Theme.monoFamily
                font.pointSize: Theme.monoSize
            }

            Text {
                width: parent.width
                text: agent.flow ? agent.flow.supplementaryMessage : ""
                visible: text !== ""
                color: agent.flow && agent.flow.supplementaryIsError ? Theme.alert : Theme.muted
                wrapMode: Text.Wrap
                font.family: Theme.monoFamily
                font.pointSize: Theme.monoSize
            }

            Prompt {
                id: prompt
                width: parent.width
                label: agent.flow ? agent.flow.inputPrompt.trim() : ""
                secret: !(agent.flow && agent.flow.responseVisible)
                onAccepted: {
                    agent.flow.submit(text);
                    clear();
                }
                onCancelled: {
                    agent.flow.cancelAuthenticationRequest();
                    clear();
                }
            }
        }
    }
}
