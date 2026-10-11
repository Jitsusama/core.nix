// The launcher on Super+Space, the desktop's command palette: type what you
// want, a window, an application, an action or what a key does, pick it
// with the arrows, and Return runs it, or Shift+Return opens it below the
// focused window. Tab takes the completion the command line offers. Each
// action shows its chord, so keys are learned by use. It lives in the shell
// and only shows and hides, so it opens as fast as niri can draw it.
//
// What it finds comes from its providers, one file each in launcher/, in
// the order jitsusama.quickshell.launcher.providers lists them (rendered
// into Providers.qml), so a kind of thing is added or removed in one line.
//
// The right arrow at the end of the line steps into the selected row: what
// can be done to it, its verbs, searched the same way, and for a verb that
// needs one, its target. What's held shows as chips in the field; the left
// arrow, or Backspace with nothing typed, steps back out, and Escape always
// closes the launcher.
//
// It's the palette kind (docs/design.md, Shape and Kinds): the accent frame,
// since it holds the keys; a command line over a list, split by a hairline;
// every row a glyph in its column, a name, and its chord or what it is.
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import QtQuick
// Quickshell makes a folder's files types only once something imports it,
// and the providers are made from launcher/ by name, so it's imported here.
import qs.launcher
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

    // The providers, made from their files as the shell starts, each given
    // its settings and weight; one that fails to load is said and skipped.
    property var sources: []

    Component.onCompleted: {
        const made = [];
        for (const listed of Providers.providers) {
            const file = Qt.createComponent(`launcher/${listed.file}`);
            if (file.status !== Component.Ready) {
                console.error(`The launcher's ${listed.name} couldn't load: ${file.errorString()}`);
                continue;
            }
            const source = file.createObject(root, {
                settings: listed,
                weight: listed.weight,
                launcher: root
            });
            source.answered.connect(root.reanswer);
            made.push(source);
        }
        sources = made;
    }

    // What the first provider to answer makes of what's typed, or null.
    // The selection starts on its headline, at -1, above the rows.
    property var answer: null
    // What the search found, below an answer's rows.
    property var found: []

    function answering(letters) {
        if (held.length > 0 || letters.trim() === "")
            return null;
        for (const source of sources) {
            const said = source.answer(letters);
            if (said)
                return said;
        }
        return null;
    }

    // An answer's rows above what was found. Asked afresh, the selection
    // goes to the top; when a late answer arrives, a selection already
    // moved stays on the row it was on.
    function compose(fresh) {
        const before = answer ? answer.rows.length : 0;
        answer = answering(typed);
        const rows = answer ? answer.rows : [];
        matches = rows.concat(placed(found, gathered));
        if (fresh || (current === -1 && !answer))
            current = answer ? -1 : 0;
        else if (current >= 0)
            current = Math.max(0, Math.min(current + rows.length - before, matches.length - 1));
    }

    function reanswer() {
        if (open && held.length === 0)
            compose(false);
    }

    // The headline as a row of its own, for Return and the footer.
    readonly property var headline: !answer ? null : {
        name: answer.headline,
        // An answer still being asked for has nothing to copy yet.
        verb: (answer.copy ?? answer.headline) === "" ? "close" : "copy",
        now: true,
        note: answer.note ?? "",
        run: () => {
            const copy = answer.copy ?? answer.headline;
            if (copy !== "")
                Quickshell.clipboardText = copy;
        }
    }
    readonly property var selected: current === -1 ? headline : matches[current] ?? null

    readonly property var choices: sources.filter(source => !source.collection)
        .reduce((all, source) => all.concat(source.things), [])
    readonly property var collections: sources.filter(source => source.collection)

    // A row for each collection that has matches: its best, with what's been
    // picked for this query before put first, and a count of the rest. Its
    // verbs are the matches, which the right arrow steps into with what was
    // typed kept. One letter finds too much of a collection to be worth a
    // row.
    property var gathered: []

    // What was found with each collection's row among it: below the rows
    // matched as tidily or picked before, above any matched more loosely,
    // so "tada" is the party popper, not a scattered match for a command.
    function placed(found, rows) {
        const all = found.slice();
        for (const row of rows) {
            const at = all.findIndex(f => !f.kept && !f.from && f.grade > row.grade);
            all.splice(at < 0 ? all.length : at, 0, row);
        }
        return all;
    }

    function gathering(letters) {
        if (letters.replace(/\s+/g, "").length < 2)
            return [];
        const rows = [];
        for (const source of collections) {
            // One opened only by its own chord adds no row to a search.
            if (source.collection.searched === false)
                continue;
            const said = source.search(letters);
            const found = learnedFirst(said.rows, letters);
            if (found.length === 0)
                continue;
            const best = found[0];
            const noun = source.collection.noun;
            rows.push(Object.assign({}, best, {
                side: found.length > 1 ? `${found.length - 1} more ${noun}` : noun,
                chip: noun,
                ask: source.collection.ask,
                lead: source.collection.glyph,
                leadHue: "voice",
                from: source,
                // One picked before for this query is as tidy as can be.
                grade: found[0] === said.rows[0] ? said.grade : 0,
                verbs: () => found
            }));
        }
        return rows;
    }

    // A collection's matches with those picked before for what's typed
    // first, most picked first, the rest in their order.
    function learnedFirst(found, letters) {
        const learned = picks.weights(letters);
        const before = found.filter(row => learned[row.key]);
        if (before.length === 0)
            return found;
        before.sort((a, b) => learned[b.key].weight - learned[a.key].weight);
        return before.concat(found.filter(row => !learned[row.key]));
    }

    // Shift+Return, as a chord beside a verb that does what it does.
    readonly property var belowChord: [
        { text: "\u21e7", wide: false },
        { text: "\u21b5", wide: false }
    ]

    // What's been stepped into, each with what was typed before and the rows
    // it offers: a thing and its verbs, then a verb and its targets.
    property var held: []
    readonly property var offered: held.length > 0 ? held[held.length - 1].rows : []

    // Steps into a row: a thing's verbs, or a verb's targets. Says whether
    // there was anywhere to go.
    function enter(row) {
        if (!row)
            return false;
        const into = held.length === 0 ? (row.verbs ? row.verbs() : null)
            : row.targets ? row.targets() : null;
        if (!into || into.length === 0)
            return false;
        held = held.concat([{ row: row, typed: typed, rows: into }]);
        // A collection is stepped into with what was typed still searching it.
        restart(row.from ? typed : "");
        return true;
    }

    function back() {
        if (held.length === 0)
            return;
        const left = held[held.length - 1];
        held = held.slice(0, -1);
        restart(left.typed);
    }

    // Searches afresh for what the field is set to, since what was found
    // before belongs to another step.
    function restart(text) {
        typed = "";
        candidates = [];
        if (prompt.text === text)
            search(text);
        else
            prompt.text = text;
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
        learns: Providers.learning.enable
        halfLife: Providers.learning.halfLife
        remember: Providers.learning.remember
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
        if (held.length > 0) {
            const from = held[held.length - 1].row.from;
            // A collection searches itself, all of it until something's typed.
            if (from)
                matches = words.length === 0 ? from.everything()
                    : learnedFirst(from.search(letters).rows, letters);
            // A step's rows, all of them in their order until something's
            // typed, then those it finds, best first.
            else if (words.length === 0)
                matches = offered.slice();
            else {
                const found = [];
                for (const row of offered) {
                    const m = Fuzzy.match(row.prepared, words, false);
                    if (m.score >= 0)
                        found.push({
                            item: row.prepared,
                            score: m.score,
                            grade: m.grade,
                            choice: row
                        });
                }
                matches = Fuzzy.keep(found).map(f => f.choice);
            }
            answer = null;
            current = 0;
            list.positionViewAtBeginning();
            return;
        }
        gathered = gathering(letters);
        if (words.length === 0) {
            candidates = [];
            root.found = [];
        } else {
            const need = Fuzzy.letters(words);
            const found = [];
            for (const choice of narrowing ? candidates : choices) {
                const byName = (choice.prepared.mask & need) === need
                    ? Fuzzy.match(choice.prepared, words, false) : { score: -1 };
                if (byName.score >= 0) {
                    found.push({
                        item: choice.prepared,
                        score: byName.score * choice.weight,
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
                        score: byWord.score * choice.weight / 4,
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
            root.found = Fuzzy.keep(found).map(f => Object.assign({
                byWord: f.byWord === true,
                times: f.times ?? 0,
                grade: f.grade,
                kept: f.kept === true
            }, f.choice));
        }
        compose(true);
        list.positionViewAtBeginning();
    }

    onCurrentChanged: {
        if (current >= 0)
            list.positionViewAtIndex(current, ListView.Contain);
    }

    // The rest of the selected name from where what's typed starts a word in
    // it, so the command line can finish the thought.
    readonly property string completion: {
        if (current === -1 && answer)
            return ` = ${answer.headline}`;
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
        const at = choice.byWord || choice.plain ? []
            : Fuzzy.match(choice.prepared, words, true).positions ?? [];
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
        if (!choice) {
            open = false;
            return;
        }
        // A verb that needs a target asks for it.
        if (held.length > 0 && choice.targets) {
            enter(choice);
            return;
        }
        // One that opens a collection steps into it, the launcher kept open.
        if (choice.opens) {
            picks.picked(typed, choice.key);
            into(choice.opens);
            return;
        }
        // The clipboard takes text only from a program holding the keys.
        if (choice.now)
            choice.run();
        open = false;
        // Acting on a thing says it was what was meant by what was typed;
        // in a collection, the one picked from it was.
        const first = held.length > 0 ? held[0] : { row: choice, typed: typed };
        picks.picked(first.row.from ? typed : first.typed, first.row.from ? choice.key
            : first.row.key);
        if (choice.now)
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

        // Opens the launcher into a collection by its provider's name.
        function collection(name: string): void {
            root.into(name);
        }
    }

    // Opens straight into a collection, all of it shown until something's
    // typed; the left arrow steps out to the search. A name no collection
    // has opens the search.
    function into(name) {
        open = true;
        const source = collections.find(s => s.settings.name === name);
        if (!source)
            return;
        held = [{
            row: {
                key: "",
                name: source.collection.noun,
                chip: source.collection.noun,
                ask: source.collection.ask,
                lead: source.collection.glyph,
                leadHue: "voice",
                from: source
            },
            typed: "",
            rows: []
        }];
        restart("");
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
            held = [];
            restart("");
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
                        // A thing held leads the line with its own glyph.
                        readonly property var thing: root.held.length > 0
                            ? root.held[0].row : null
                        glyph: thing ? thing.lead ?? thing.glyph : String.fromCodePoint(0xf0349)
                        hue: thing ? Theme[thing.leadHue ?? thing.hue] : Theme.muted
                        glyphWidth: Theme.space.xxl
                        chips: root.held.map(step => ({ text: step.row.chip ?? step.row.name }))
                        hint: root.held.length === 0 ? "what next?"
                            : root.held[root.held.length - 1].row.ask ?? "what to do?"
                        ghost: root.completion
                        // Where the selection is, once there's more than fits.
                        aside: root.matches.length > root.rows
                            && root.current >= 0
                            ? `${root.current + 1} of ${root.matches.length}` : ""
                        onTextChanged: root.search(text)
                        onAccepted: below => root.run(root.selected, below)
                        // Tab on an answer puts the answer in place of the
                        // question, so a sum can be carried on; one still
                        // being asked for leaves the question as it is.
                        onCompleted: {
                            if (root.current !== -1 || !root.answer)
                                text = text + root.completion;
                            else if ((root.answer.copy ?? root.answer.headline) !== "")
                                text = root.answer.copy ?? root.answer.headline;
                        }
                        onCancelled: root.open = false
                        onForward: root.enter(root.matches[root.current])
                        onBackward: root.back()
                        // The highest the selection goes: an answer's headline
                        // sits above the rows, at -1.
                        readonly property int highest: root.answer ? -1 : 0
                        Keys.onUpPressed: root.current = Math.max(root.current - 1, highest)
                        Keys.onDownPressed: root.current
                            = Math.max(highest, Math.min(root.current + 1, root.matches.length - 1))
                        Keys.onPressed: event => {
                            if (event.key === Qt.Key_PageUp)
                                root.current = Math.max(root.current - root.rows, highest);
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
                            count: modelData.count ?? ""
                            onClicked: root.run(modelData, false)
                            onPointed: at => root.pointed(index, at)
                        }
                    }

                    // What Return does here, Shift+Return where it can, and
                    // where the arrows step, with why the row is first when
                    // learning put it there, where a held thing is, or the
                    // chord a target has: the one place that teaches, so the
                    // rows stay clean.
                    Item {
                        id: foot

                        readonly property var choice: root.selected

                        width: parent.width
                        height: Theme.space.xxl + Theme.space.s
                        visible: root.matches.length > 0 || root.answer !== null

                        Text {
                            x: Theme.space.l
                            anchors.verticalCenter: parent.verticalCenter
                            width: keys.x - x - Theme.space.xl
                            elide: Text.ElideRight
                            readonly property string asked: Fuzzy.words(root.typed).join(" ")
                            readonly property int times: foot.choice ? foot.choice.times ?? 0 : 0

                            // An answer's footer says where it came from.
                            text: !foot.choice ? ""
                                : (foot.choice.note ?? "") !== "" ? foot.choice.note
                                : foot.choice.plain && root.answer ? root.answer.note ?? ""
                                : root.held.length > 0 ? root.held[0].row.context
                                : times === 0 ? "" : `you picked this for \u201c${asked}\u201d ${
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
                                model: {
                                    const choice = foot.choice;
                                    if (!choice)
                                        return [];
                                    const keys = [["\u21b5", choice.verb ?? "run"]];
                                    if (root.held.length === 0 && choice.below)
                                        keys.push(["\u21e7\u21b5", "below"]);
                                    if (root.held.length === 0 && choice.verbs)
                                        keys.push(["\u2192", choice.from ? "see all" : "more"]);
                                    if (root.held.length > 0)
                                        keys.push(["\u2190", "back"]);
                                    return keys;
                                }

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
                        visible: root.matches.length === 0 && root.answer === null
                        glyph: root.typed === "" ? root.sparkle : root.ghost
                        hue: Theme.voice
                        quiet: true
                        name: root.typed === ""
                            ? "A window, a program, an action, or what a key does."
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
        // A word after the name, muted: "here" beside a workspace.
        property string count
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
            id: nameText
            x: Theme.space.l + Theme.space.xxl
            width: Math.min(implicitWidth, right.x - x - Theme.space.xxl
                - (row.count === "" ? 0 : countText.implicitWidth + Theme.space.m))
            anchors.verticalCenter: parent.verticalCenter
            text: row.name
            textFormat: row.selected || row.quiet ? Text.PlainText : Text.StyledText
            elide: Text.ElideRight
            color: row.quiet ? Theme.muted : row.selected ? Theme.strong : Theme.text
            font.family: Theme.mono.family
            font.pointSize: Theme.mono.large
            font.weight: row.selected ? Font.Medium : Font.Normal
        }

        Text {
            id: countText
            x: nameText.x + nameText.width + Theme.space.m
            anchors.verticalCenter: parent.verticalCenter
            visible: row.count !== ""
            text: row.count
            color: Theme.muted
            font.family: Theme.mono.family
            font.pointSize: Theme.mono.large
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
