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
import "Fuzzy.js" as Fuzzy

Scope {
    id: root

    required property var lock
    required property var grid
    property bool open: false

    // Wide enough for a long action and its chord at the large size.
    readonly property int width: 680
    // How many results show at once; the rest are a scroll away.
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
        key: `session:${action.name}`,
        verb: action.name.toLowerCase(),
        name: action.name,
        prepared: Fuzzy.prepare(action.name),
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
                key: `program:${entry.id}`,
                verb: "open",
                name: entry.name,
                prepared: Fuzzy.prepare(entry.name),
                glyph: String.fromCodePoint(root.programGlyphs[entry.id] ?? 0xf0614),
                hue: "program",
                chord: [],
                side: entry.genericName.toLowerCase(),
                // Found by what it is, too: "browser" finds Chrome.
                keywords: Fuzzy.prepare([entry.genericName, entry.comment]
                    .concat(Array.from(entry.keywords)).join(" ")),
                run: () => Niri.spawn(root.command(entry)),
                below: () => {
                    root.grid.below("");
                    Niri.spawn(root.command(entry));
                },
                later: false
            }));
        const actions = Shortcuts.actions.map(action => ({
            key: `action:${action.does}`,
            verb: "run",
            name: action.does,
            prepared: Fuzzy.prepare(action.does),
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

    // The words typed, and every choice they found before the finest were
    // kept: a longer query can only find fewer, so it searches these.
    property var words: []
    property var candidates: []

    // What was picked for what was typed, which the ranking leans on.
    Picks {
        id: picks
    }

    // How much one past pick for exactly this query is worth, against a
    // match's score: enough to settle a close call, as much as a good match
    // is worth, while a far better match can still win.
    readonly property int learnedWorth: 100

    // Every choice the words find, best first (Fuzzy.js says how), with
    // what was picked for this query before leaning on the order (Picks.qml
    // says how); a thing picked before is shown even when it matches more
    // loosely than the rest. One found only by what it is, its keywords,
    // ranks well below those found by name.
    function search(letters) {
        const narrowing = typed !== "" && letters.startsWith(typed);
        typed = letters;
        words = Fuzzy.words(letters);
        if (words.length === 0) {
            candidates = [];
            matches = [];
        } else {
            const need = Fuzzy.letters(words);
            const found = [];
            for (const choice of narrowing ? candidates : choices) {
                const byName = (choice.prepared.mask & need) === need
                    ? Fuzzy.match(choice.prepared, words, false) : { score: -1 };
                if (byName.score >= 0) {
                    found.push({
                        item: choice.prepared,
                        score: byName.score,
                        grade: byName.grade,
                        choice: choice
                    });
                    continue;
                }
                if (!choice.keywords || (choice.keywords.mask & need) !== need)
                    continue;
                const byWord = Fuzzy.match(choice.keywords, words, false);
                if (byWord.score >= 0)
                    found.push({
                        item: choice.prepared,
                        score: byWord.score / 4,
                        grade: byWord.grade,
                        choice: choice,
                        byWord: true
                    });
            }
            candidates = found.map(f => f.choice);
            const learned = picks.weights(letters);
            for (const f of found) {
                const before = learned[f.choice.key];
                if (!before)
                    continue;
                f.score += learnedWorth * before.weight;
                f.kept = true;
                f.times = before.times;
            }
            matches = Fuzzy.keep(found).map(f =>
                Object.assign({ byWord: f.byWord === true, times: f.times ?? 0 }, f.choice));
        }
        current = 0;
        list.positionViewAtBeginning();
    }

    onCurrentChanged: list.positionViewAtIndex(current, ListView.Contain)

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
    // Worked out only for the rows drawn.
    function marked(choice) {
        const escape = text => text.replace(/&/g, "&amp;").replace(/</g, "&lt;");
        const at = choice.byWord ? [] : Fuzzy.match(choice.prepared, words, true).positions ?? [];
        let out = "";
        for (let i = 0; i < choice.name.length; i++) {
            const letter = escape(choice.name[i]);
            out += at.includes(i)
                ? `<font color="${Theme.strong}">${letter}</font>`
                : letter;
        }
        return out;
    }

    function run(choice, below) {
        open = false;
        if (!choice)
            return;
        picks.picked(typed, choice.key);
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

    // Where the pointer last was, in the window, so the selection follows
    // it only when it moves: a launcher opening under a resting pointer, or
    // a list scrolling under one, leaves the selection where the keys put it.
    property point pointer: Qt.point(-1, -1)

    function pointed(index, at) {
        const moved = pointer.x >= 0 && (at.x !== pointer.x || at.y !== pointer.y);
        pointer = at;
        if (moved)
            current = index;
    }

    onOpenChanged: {
        if (open) {
            pointer = Qt.point(-1, -1);
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
                        // Where the selection is, once there's more than fits.
                        aside: root.matches.length > root.rows
                            ? `${root.current + 1} of ${root.matches.length}` : ""
                        onTextChanged: root.search(text)
                        onAccepted: below => root.run(root.matches[root.current], below)
                        onCompleted: text = text + root.completion
                        onCancelled: root.open = false
                        Keys.onUpPressed: root.current = Math.max(root.current - 1, 0)
                        Keys.onDownPressed: root.current
                            = Math.min(root.current + 1, root.matches.length - 1)
                        Keys.onPressed: event => {
                            if (event.key === Qt.Key_PageUp)
                                root.current = Math.max(root.current - root.rows, 0);
                            else if (event.key === Qt.Key_PageDown)
                                root.current
                                    = Math.min(root.current + root.rows, root.matches.length - 1);
                        }
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

                    ListView {
                        id: list

                        width: parent.width
                        height: Math.min(count, root.rows) * Theme.space.xxxl
                        visible: count > 0
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        model: root.matches

                        delegate: Choice {
                            required property var modelData
                            required property int index

                            width: list.width
                            selected: index === root.current
                            glyph: modelData.glyph
                            hue: Theme[modelData.hue]
                            name: selected ? modelData.name : root.marked(modelData)
                            chord: modelData.chord
                            side: modelData.side
                            onClicked: root.run(modelData, false)
                            onPointed: at => root.pointed(index, at)
                        }
                    }

                    // What Return does here, and Shift+Return where it can,
                    // with why the row is first when learning put it there:
                    // the one place that teaches, so the rows stay clean.
                    Item {
                        id: foot

                        readonly property var choice: root.matches[root.current]

                        width: parent.width
                        height: Theme.space.xxl + Theme.space.s
                        visible: root.matches.length > 0

                        Text {
                            x: Theme.space.l
                            anchors.verticalCenter: parent.verticalCenter
                            width: keys.x - x - Theme.space.xl
                            elide: Text.ElideRight
                            readonly property string asked: Fuzzy.words(root.typed).join(" ")
                            readonly property int times: foot.choice ? foot.choice.times : 0

                            text: times === 0 ? "" : `you picked this for \u201c${asked}\u201d ${
                                times === 1 ? "once" : `${times} times`}`
                            color: Theme.muted
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.large
                        }

                        Row {
                            id: keys
                            anchors.right: parent.right
                            anchors.rightMargin: Theme.space.l
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: Theme.space.xl

                            Repeater {
                                model: !foot.choice ? []
                                    : [["\u21b5", foot.choice.verb]].concat(
                                        foot.choice.below ? [["\u21e7\u21b5", "below"]] : [])

                                Text {
                                    required property var modelData

                                    text: `<font color="${Theme.text}">${modelData[0]}</font> `
                                        + modelData[1]
                                    textFormat: Text.StyledText
                                    color: Theme.muted
                                    font.family: Theme.mono.family
                                    font.pointSize: Theme.mono.large
                                }
                            }
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
        // The pointer is over the row, at this place in the window.
        signal pointed(point at)

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
            onPositionChanged: mouse => row.pointed(mapToItem(null, mouse.x, mouse.y))
        }
    }
}
