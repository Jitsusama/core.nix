// The launcher on Super+Space: type a few letters of an application or an
// action, pick it with the arrows, and Return runs it. It lives in the shell
// and only shows and hides, so it opens as fast as the compositor can draw it.
// Ending the session lives here rather than on a key, so no chord ends it by
// accident.
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import QtQuick

Scope {
    id: root

    required property var lock
    property bool open: false

    readonly property var actions: [
        { name: "Lock", run: () => root.lock.lock() },
        { name: "Suspend", run: () => Niri.spawn(["systemctl", "suspend"]) },
        { name: "Log Out", run: () => Niri.act({ Quit: { skip_confirmation: true } }) },
        { name: "Reboot", run: () => Niri.spawn(["systemctl", "reboot"]) },
        { name: "Power Off", run: () => Niri.spawn(["systemctl", "poweroff"]) }
    ]

    readonly property var choices: {
        const applications = DesktopEntries.applications.values
            .filter(entry => !entry.noDisplay)
            .map(entry => ({ name: entry.name, run: () => Niri.spawn(root.command(entry)) }));
        return applications.concat(actions);
    }

    // What an application's entry runs, spelled out for niri to spawn: in
    // kitty when the entry asks for a terminal, and from its own directory
    // when it names one.
    function command(entry: var): var {
        const program = entry.runInTerminal ? ["kitty"].concat(entry.command) : entry.command;
        if (entry.workingDirectory === "")
            return program;
        return ["sh", "-c", "cd -- \"$0\" && exec \"$@\"", entry.workingDirectory].concat(program);
    }

    property var matches: []

    // How well a choice's name matches what's typed: every letter has to
    // appear in order, and letters that start a word or follow one another
    // count for more. Zero means no match.
    function score(name, typed) {
        const haystack = name.toLowerCase();
        let total = 0;
        let from = 0;
        let previous = -2;
        for (const letter of typed.toLowerCase()) {
            const at = haystack.indexOf(letter, from);
            if (at < 0)
                return 0;
            total += 1;
            if (at === 0 || haystack[at - 1] === " ")
                total += 2;
            if (at === previous + 1)
                total += 1;
            previous = at;
            from = at + 1;
        }
        return total - haystack.length / 100;
    }

    function search(typed) {
        matches = choices
            .map(choice => ({ choice: choice, score: typed === "" ? 1 : score(choice.name, typed) }))
            .filter(match => match.score > 0)
            .sort((a, b) => b.score - a.score || a.choice.name.localeCompare(b.choice.name))
            .slice(0, 8)
            .map(match => match.choice);
        list.currentIndex = 0;
    }

    function run(choice) {
        open = false;
        if (choice)
            choice.run();
    }

    IpcHandler {
        target: "launcher"

        function toggle(): void {
            root.open = !root.open;
        }
    }

    onOpenChanged: {
        if (open) {
            prompt.clear();
            search("");
        }
    }

    PanelWindow {
        visible: root.open
        anchors.top: true
        margins.top: screen ? screen.height / 4 : 0
        implicitWidth: 600
        implicitHeight: box.implicitHeight
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
        WlrLayershell.namespace: "launcher"

        Column {
            id: box
            width: parent.width

            Prompt {
                id: prompt
                width: parent.width
                label: "Run"
                onTextChanged: root.search(text)
                onAccepted: root.run(root.matches[list.currentIndex])
                onCancelled: root.open = false
                Keys.onUpPressed: list.decrementCurrentIndex()
                Keys.onDownPressed: list.incrementCurrentIndex()
            }

            Rectangle {
                width: parent.width
                height: list.contentHeight + (root.matches.length > 0 ? Theme.space.xl : 0)
                visible: root.matches.length > 0
                color: Theme.background
                border.color: Theme.accent
                border.width: Theme.border
                radius: Theme.radius

                ListView {
                    id: list
                    anchors.fill: parent
                    anchors.margins: Theme.space.m
                    interactive: false
                    model: root.matches

                    delegate: Rectangle {
                        id: row

                        required property var modelData
                        required property int index

                        width: list.width
                        height: name.implicitHeight + Theme.space.m
                        color: ListView.isCurrentItem ? Theme.selection : "transparent"
                        radius: Theme.radius

                        Text {
                            id: name
                            anchors.left: parent.left
                            anchors.leftMargin: Theme.space.m
                            anchors.verticalCenter: parent.verticalCenter
                            text: row.modelData.name
                            color: row.ListView.isCurrentItem ? Theme.strong : Theme.text
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.body
                        }

                        MouseArea {
                            anchors.fill: parent
                            onClicked: root.run(row.modelData)
                        }
                    }
                }
            }
        }
    }
}
