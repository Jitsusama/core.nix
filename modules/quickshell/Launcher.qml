// The launcher on Super+Space, the desktop's command palette: type what you
// want, an application, an action or what a key does, pick it with the
// arrows, and Return runs it, or Shift+Return opens it below the focused
// window. Tab takes the completion the command line offers. Each action
// shows its chord, so keys are learned by use. Ending the session lives here
// rather than on a key, so no chord ends it by accident. It lives in the
// shell and only shows and hides, so it opens as fast as niri can draw it.
//
// It's the palette kind (docs/design.md, Shape and Kinds): the accent frame,
// since it holds the keys; a command line over a list, split by a hairline;
// every row a glyph in its column, a name, and its chord or what it is.
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import QtQuick

Scope {
    id: root

    required property var lock
    required property var grid
    property bool open: false

    // Wide enough for a long action and its chord at the large size.
    readonly property int width: 680
    readonly property int rows: 8

    readonly property string sparkle: String.fromCodePoint(0xf0674)
    readonly property string ghost: String.fromCodePoint(0xf02a0)

    readonly property var session: [
        { name: "Lock", glyph: 0xf023, run: () => root.lock.lock() },
        { name: "Suspend", glyph: 0xf04b2, run: () => Niri.spawn(["systemctl", "suspend"]) },
        {
            name: "Log Out",
            glyph: 0xf0343,
            run: () => Niri.act({ Quit: { skip_confirmation: true } })
        },
        { name: "Reboot", glyph: 0xf0453, run: () => Niri.spawn(["systemctl", "reboot"]) },
        { name: "Power Off", glyph: 0xf011, run: () => Niri.spawn(["systemctl", "poweroff"]) }
    ].map(action => ({
        name: action.name,
        glyph: String.fromCodePoint(action.glyph),
        hue: "machine",
        chord: [],
        side: "",
        run: action.run,
        later: false
    }))

    // An application's own glyph where Nerd Fonts has one, by its entry's id.
    readonly property var programGlyphs: ({
        "kitty": 0xf011b,
        "google-chrome": 0xf268,
        "slack": 0xf198,
        "nvim": 0xf36f,
        "1password": 0xf0881
    })

    readonly property var choices: {
        const applications = DesktopEntries.applications.values
            .filter(entry => !entry.noDisplay)
            .map(entry => ({
                name: entry.name,
                glyph: String.fromCodePoint(root.programGlyphs[entry.id] ?? 0xf0614),
                hue: "program",
                chord: [],
                side: entry.genericName.toLowerCase(),
                run: () => Niri.spawn(root.command(entry)),
                below: () => {
                    root.grid.below("");
                    Niri.spawn(root.command(entry));
                },
                later: false
            }));
        const actions = Shortcuts.actions.map(action => ({
            name: action.does,
            glyph: action.glyph,
            hue: "action",
            chord: action.chord,
            side: "",
            run: () => Niri.spawn(action.run),
            // Acts on the screen, so it waits for the launcher to be gone.
            later: true
        }));
        return applications.concat(actions, session);
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

    property string typed: ""
    property var matches: []
    property int current: 0
    property var pending: null

    // Where each typed letter lands in a name, in order, or null when one
    // doesn't. A letter that starts a word or follows the last one counts
    // for more, so the best match comes first.
    function find(name, letters) {
        const haystack = name.toLowerCase();
        const at = [];
        let total = 0;
        let from = 0;
        let previous = -2;
        for (const letter of letters.toLowerCase()) {
            const found = haystack.indexOf(letter, from);
            if (found < 0)
                return null;
            at.push(found);
            total += 1;
            if (found === 0 || haystack[found - 1] === " ")
                total += 2;
            if (found === previous + 1)
                total += 1;
            previous = found;
            from = found + 1;
        }
        return { at: at, score: total - haystack.length / 100 };
    }

    function search(letters) {
        typed = letters;
        matches = letters === "" ? [] : choices
            .map(choice => ({ choice: choice, found: find(choice.name, letters) }))
            .filter(match => match.found !== null)
            .sort((a, b) => b.found.score - a.found.score
                || a.choice.name.localeCompare(b.choice.name))
            .slice(0, rows)
            .map(match => Object.assign({ at: match.found.at }, match.choice));
        current = 0;
    }

    // The rest of the selected name from where what's typed starts a word in
    // it, so the command line can finish the thought.
    readonly property string completion: {
        const choice = matches[current];
        if (!choice || typed === "")
            return "";
        const name = choice.name;
        const at = name.toLowerCase().indexOf(typed.toLowerCase());
        if (at < 0 || (at > 0 && name[at - 1] !== " "))
            return "";
        return name.slice(at + typed.length);
    }

    // A name with the letters that matched brought forward, as styled text.
    function marked(choice) {
        const escape = text => text.replace(/&/g, "&amp;").replace(/</g, "&lt;");
        let out = "";
        for (let i = 0; i < choice.name.length; i++) {
            const letter = escape(choice.name[i]);
            out += choice.at.includes(i)
                ? `<font color="${Theme.strong}">${letter}</font>`
                : letter;
        }
        return out;
    }

    function run(choice, below) {
        open = false;
        if (!choice)
            return;
        if (below && choice.below)
            choice.below();
        else if (choice.later) {
            pending = choice;
            later.restart();
        } else
            choice.run();
    }

    Timer {
        id: later
        interval: Theme.motion.short
        onTriggered: {
            root.pending.run();
            root.pending = null;
        }
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
        margins.top: screen ? screen.height / 4 - card.reach : 0
        implicitWidth: root.width + 2 * card.reach
        implicitHeight: card.height + 2 * card.reach
        exclusionMode: ExclusionMode.Ignore
        color: "transparent"
        mask: card.area
        BackgroundEffect.blurRegion: card.area
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
        WlrLayershell.namespace: "launcher"

        Floating {
            id: card
            x: reach
            y: reach
            width: root.width
            keys: true

            Column {
                width: parent.width

                // What's typed, an inset inside the frame.
                Item {
                    width: parent.width
                    height: prompt.height + 2 * Theme.inset

                    Prompt {
                        id: prompt
                        x: Theme.inset
                        y: Theme.inset
                        width: parent.width - 2 * Theme.inset
                        glyph: String.fromCodePoint(0xf0349)
                        glyphWidth: Theme.space.xxl
                        hint: "what next?"
                        ghost: root.completion
                        onTextChanged: root.search(text)
                        onAccepted: below => root.run(root.matches[root.current], below)
                        onCompleted: text = text + root.completion
                        onCancelled: root.open = false
                        Keys.onUpPressed: root.current = Math.max(root.current - 1, 0)
                        Keys.onDownPressed: root.current
                            = Math.min(root.current + 1, root.matches.length - 1)
                    }
                }

                // What it reaches, split from what's typed by a hairline.
                Rectangle {
                    width: parent.width
                    height: Theme.border
                    color: Theme.edge
                }

                Column {
                    x: Theme.inset
                    width: parent.width - 2 * Theme.inset
                    topPadding: Theme.inset
                    bottomPadding: Theme.inset

                    Repeater {
                        model: root.matches

                        Choice {
                            required property var modelData
                            required property int index

                            selected: index === root.current
                            glyph: modelData.glyph
                            hue: Theme[modelData.hue]
                            name: selected ? modelData.name : root.marked(modelData)
                            chord: modelData.chord
                            side: modelData.side
                            onClicked: root.run(modelData, false)
                            onHovered: root.current = index
                        }
                    }

                    // With nothing typed, the shell says what it can do; with
                    // nothing found, it says so.
                    Choice {
                        visible: root.matches.length === 0
                        glyph: root.typed === "" ? root.sparkle : root.ghost
                        hue: Theme.voice
                        quiet: true
                        name: root.typed === ""
                            ? "A program, an action, or what a key does."
                            : `Nothing answers to \u201c${root.typed}\u201d.`
                    }
                }
            }
        }
    }

    // A row of the list: its glyph in a column of its own, its name, and on
    // the right its chord or what it is, muted.
    // Not Row, which is QtQuick's own and would silently win.
    component Choice: Rectangle {
        id: row

        property bool selected: false
        property string glyph
        property color hue
        property string name
        property var chord: []
        property string side
        // Said by the shell rather than found, so it's muted and can't be picked.
        property bool quiet: false

        signal clicked
        signal hovered

        width: parent.width
        height: Theme.space.xxxl
        radius: Theme.inner
        color: selected ? Qt.alpha(Theme.selection, Theme.floating.selection) : "transparent"

        Text {
            x: Theme.space.l
            anchors.verticalCenter: parent.verticalCenter
            text: row.glyph
            color: row.hue
            font.family: Theme.mono.family
            font.pointSize: Theme.mono.large
        }

        Text {
            x: Theme.space.l + Theme.space.xxl
            width: right.x - x - Theme.space.xxl
            anchors.verticalCenter: parent.verticalCenter
            text: row.name
            textFormat: row.selected || row.quiet ? Text.PlainText : Text.StyledText
            elide: Text.ElideRight
            color: row.quiet ? Theme.muted : row.selected ? Theme.strong : Theme.text
            font.family: Theme.mono.family
            font.pointSize: Theme.mono.large
            font.weight: row.selected ? Font.Medium : Font.Normal
        }

        // The chord key by key, joined by +, or what the row is.
        Item {
            id: right
            anchors.right: parent.right
            anchors.rightMargin: Theme.space.l
            anchors.verticalCenter: parent.verticalCenter
            width: keys.width
            height: keys.height

            Flow {
                id: keys

                Repeater {
                    model: row.chord.length > 0 ? row.chord
                        : row.side === "" ? [] : [{ text: row.side, wide: false }]

                    Text {
                        required property var modelData
                        required property int index

                        // A part takes the room its glyph draws into, and each
                        // + keeps the same distance from both its keys.
                        leftPadding: index === 0 ? 0 : plus.width
                        rightPadding: modelData.wide ? Theme.space.xs : 0
                        text: modelData.text
                        color: Theme.muted
                        font.family: Theme.mono.family
                        font.pointSize: Theme.mono.large

                        Text {
                            id: plus
                            visible: index > 0
                            width: implicitWidth + 2 * Theme.space.xs * 3 / 2
                            horizontalAlignment: Text.AlignHCenter
                            text: "+"
                            color: Theme.muted
                            font: parent.font
                        }
                    }
                }
            }
        }

        MouseArea {
            anchors.fill: parent
            enabled: !row.quiet
            hoverEnabled: true
            onClicked: row.clicked()
            onEntered: row.hovered()
        }
    }
}
