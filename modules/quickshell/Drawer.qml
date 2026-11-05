// The launcher's drawer: what the selected row has to show beyond its one
// line, out of the card's right side rather than below its rows (Joel,
// 2026-10-28: "having it come out of the bottom feels wrong, i think it
// should come out of the right side like a drawer, making sure it never
// passes the bound of the screen"). It's the card continued, a floating
// surface like it, as tall as the card and top aligned with it, so the
// card never changes size for what's selected.
//
// It's set as a plate, a small page about the selected thing (look 3's
// piece 3), ranked by its type (docs/design.md, Type): an eyebrow saying
// when or where at Label, the thing's name at Title, the one strong thing
// while it's shown, its facts as a label at Note and a value at Figure,
// its words, then its picture. Every line of it lies on one of the card's lines, so the two
// read as one page: the eyebrow on the field's centre, the name on the
// first row's baseline, and the rest on a pitch of half a row from the
// second row's, one empty line between each group. A row gives it as
// `drawer: { eyebrow, title, preview, facts: [{ label, value }], body,
// styled, picture, across, down }`, the body plain unless it says it's
// styled, a fact whose value isn't known yet null, shown as a fixed
// placeholder until it is. A preview, as a file's plate has (FileFacts.js),
// comes right under the name: `{ picture, across, down }` or `{ lines,
// quiet }`. Shift+Up and Shift+Down scroll it a few lines at a time,
// as the launcher's foot says while it holds more than fits
// (docs/design.md, the drawer).
//
// Home's Now gives a plate of the moment instead (look 3's piece 4,
// launcher/WhatsOn.qml): `{ now: true, eyebrow, seconds, say, groups:
// [[{ label, value, qualifier }]], lines: { say, groups } }`. The time is
// large on Now's row, its seconds counting after it, what needs Joel on
// the row `lines.say` names, and each group of facts that holds any from
// the next row `lines.groups` names, on the pitch; a row that's missing,
// -1, sets its part one empty line below the one before.
pragma ComponentBehavior: Bound
import Quickshell
import QtQuick
import QtQuick.Effects
import "launcher/Moment.js" as Moment

Floating {
    id: drawer

    // What it's given to show, or null once it's to go in, when it keeps
    // what it last showed while it slides away.
    property var given: null
    property var shown: null
    onGivenChanged: {
        if (given)
            shown = given;
    }
    // Whether it's out, which it fades only for, never for what it shows.
    property bool out: false
    // Out again, it shows what it was last given, which a row coming back
    // hands it unchanged: what it showed was let go as it faded, and the
    // same drawer given again changes nothing to show it by.
    onOutChanged: {
        if (out && given)
            shown = given;
    }
    // How tall it is, the card's height.
    property real tall: 0
    // The card's lines it's set on, down from the card's top as from its
    // own, since the two share a top: the field's centre and the first
    // row's.
    property real fieldLine: 0
    property real rowLine: 0
    // The middle of each row the card shows, for the moment's lines.
    property var rowLines: []

    readonly property string eyebrow: shown ? shown.eyebrow ?? "" : ""
    readonly property string title: shown ? shown.title ?? "" : ""
    // Its facts, less any that only says the title again, as a server's
    // name would right under it (VKCF3Z, 2026-11-03).
    readonly property var facts: shown ? (shown.facts ?? [])
        .filter(fact => fact && String(fact.value ?? "") !== title) : []
    readonly property string body: shown ? shown.body ?? "" : ""
    readonly property string picture: shown ? shown.picture ?? "" : ""
    readonly property var preview: shown ? shown.preview ?? null : null
    // Whether it's the moment's plate, home's Now's.
    readonly property bool moment: shown?.now === true
    // Whether its words are prose to read, set as the reader sets its text,
    // rather than lines in the mono.
    readonly property bool prose: shown?.prose === true
    readonly property var momentLines: moment ? shown.lines ?? {} : ({})
    // Its groups of facts that hold any, each set on the row the next of
    // `lines.groups` names: a group with nothing to show gives up its
    // place, so a day with no weather or meeting leaves no empty lines
    // in the middle of the plate (docs/design.md, Space Available).
    readonly property var groups: moment ? (shown.groups ?? [])
        .filter(facts => facts.length > 0)
        .map((facts, at) => ({ facts: facts, line: (momentLines.groups ?? [])[at] ?? -1 }))
        : []
    // The time and its seconds as it says them, for the launcher to report.
    readonly property string momentTime: moment ? timeText.text : ""
    readonly property string momentSeconds: moment ? secondsText.text : ""
    // Whether it holds more than fits, so the foot says how to scroll it.
    readonly property bool more: view.contentHeight > view.height + 1
    // How far it's scrolled, for the launcher to report.
    readonly property real scrolled: view.contentY

    // The plate's pitch: half a row, so its lines fall on each row's and
    // halfway between two.
    readonly property real pitch: Theme.space.xl
    // How far below a line's centre a row's name has its baseline, as a row
    // centred on that line does, and the same at Label for the eyebrow. Every line here is set on such a
    // baseline, whatever its own face, since the faces' line boxes differ.
    readonly property real dropped: monoRuler.y + monoRuler.baselineOffset
    readonly property real eyebrowDropped: eyebrowRuler.y + eyebrowRuler.baselineOffset

    // Where its lines' baselines are, down from its top, for the launcher
    // to report: the eyebrow's and the middle of its line, the title's first
    // and last lines',
    // each fact's first line's, the words' first line's, and the top of a
    // preview and its lines' baselines. Worked out
    // only when asked, since nothing drawn depends on them.
    function baselines() {
        const contentTop = Theme.border - view.contentY;
        const shown = item => item.visible ? contentTop + item.y + item.baselineOffset : null;
        const flowed = item => item.visible ? contentTop + flow.y + item.y + item.baselineOffset
            : null;
        return {
            eyebrow: shown(eyebrowText),
            eyebrowMiddle: eyebrowText.visible ? contentTop + eyebrowText.y
                + eyebrowText.height / 2 : null,
            words: wordsBox.visible ? contentTop + flow.y + wordsBox.y + bodyText.y
                + bodyText.baselineOffset : null,
            title: flowed(titleText),
            // Its last line's, two of the pitch a line, which what follows
            // stands one empty line below.
            titleLast: titleText.visible ? flowed(titleText)
                + (titleText.lineCount - 1) * 2 * drawer.pitch : null,
            preview: previewBox.visible ? contentTop + flow.y + previewBox.y : null,
            previewLines: previewLines.visible ? Array.from({ length: previewRows.count },
                (_, at) => {
                    const line = previewRows.itemAt(at);
                    return line ? contentTop + flow.y + previewBox.y + line.y
                        + line.baselineOffset : null;
                }) : [],
            facts: factsShown.visible ? Array.from({ length: factRows.count }, (_, at) => {
                const row = factRows.itemAt(at);
                return row ? contentTop + flow.y + factsShown.y + row.y + row.base : null;
            }) : [],
            // The moment's: the time's and its seconds', what needs Joel's,
            // and each group's facts'.
            time: drawer.moment ? shown(timeText) : null,
            timeMiddle: drawer.moment ? shown(timeText)
                - timeFace.tightBoundingRect("H").height / 2 : null,
            seconds: drawer.moment ? shown(secondsText) : null,
            say: drawer.moment ? shown(sayText) : null,
            groups: drawer.moment ? Array.from({ length: groupsShown.count }, (_, at) => {
                const group = groupsShown.itemAt(at);
                return group ? group.bases.map(base => contentTop + group.y + base) : [];
            }) : []
        };
    }

    // A row's line of the card, by its place in the card's rows, its middle.
    function lineOf(at) {
        return at >= 0 && at < rowLines.length ? rowLines[at] : null;
    }
    // The baseline a line of the mono takes on a row's line, or one empty
    // line below where the part before ended where that row is missing.
    function rowBase(at, after) {
        const line = lineOf(at);
        return line !== null ? line + dropped : after + 2 * pitch;
    }
    // A figure, then what qualifies it, quieter after a dot, each number
    // kept to its unit.
    function qualified(fact) {
        const plain = words => unbroken(words ?? "").replace(/&/g, "&amp;")
            .replace(/</g, "&lt;");
        return (fact.qualifier ?? "") === "" ? plain(fact.value)
            : `${plain(fact.value)} <font color="${Theme.muted}">\u00b7 `
                + `${plain(fact.qualifier)}</font>`;
    }

    // Moves what it shows three lines up or down, never past either end,
    // so its lines stay on the card's.
    function scroll(lines) {
        view.contentY = Math.max(0, Math.min(view.contentHeight - view.height,
            view.contentY + lines * 3 * pitch));
    }

    height: tall
    opacity: out ? 1 : 0
    visible: opacity > 0
    // Gone once it has faded away, so what it showed is let go: a picture
    // kept while hidden would be read again as the screen's scale changes,
    // after its file may be gone.
    onVisibleChanged: {
        if (!visible && !out)
            shown = null;
    }
    // It fades in where it rests and out again, on the one curve, never
    // sliding, so nothing beside the card moves as the selection does
    // (VKCF3Z, 2026-10-29).
    Behavior on opacity {
        NumberAnimation {
            duration: drawer.out ? Theme.motion.medium : Theme.motion.short
            easing.type: Easing.BezierSpline
            easing.bezierCurve: Theme.motion.curve
        }
    }
    // Each new thing it shows starts at its top; the same thing read anew,
    // as a row refreshes, stays where it was scrolled.
    onTitleChanged: view.contentY = 0

    // Empty lines of the mono, centred on a line's centre as a row's words
    // are, whose baselines say where the plate's are: an empty line keeps
    // the baseline a filled one has.
    Text {
        id: monoRuler
        y: -height / 2
        visible: false
        font.family: Theme.level.name.family
        font.pointSize: Theme.level.name.pointSize
    }
    Text {
        id: eyebrowRuler
        y: -height / 2
        visible: false
        font.family: Theme.level.label.family
        font.pointSize: Theme.level.label.pointSize
    }

    // The widest of its facts' names, which their values start beyond, so
    // the values share one edge: a name is never cut short, and a value
    // wraps instead (VKCF3Z, 2026-11-03). Only one so long it would leave
    // its figure less than eight figures' room breaks onto a second line.
    FontMetrics {
        id: factFace
        font.family: Theme.level.note.family
        font.pointSize: Theme.level.note.pointSize
    }
    FontMetrics {
        id: valueFace
        font.family: Theme.level.figure.family
        font.pointSize: Theme.level.figure.pointSize
    }
    // Every fact it shows, the moment's groups' among them.
    readonly property var allFacts: facts.concat(...groups.map(group => group.facts))
    readonly property real labelsWide: Math.min(
        column.width - labelsGap - valueFace.advanceWidth("00000000"),
        Math.ceil(Math.max(0, ...allFacts.map(fact => factFace.advanceWidth(fact.label ?? "")))))
    // Whether every name fits its column on one line, for the launcher to
    // report when asked.
    function labelsWhole() {
        return allFacts.every(fact => factFace.advanceWidth(fact.label ?? "") <= labelsWide);
    }
    // Where the last line it shows ends, down from its top.
    readonly property real foot: Theme.border + view.height
    // Between a name and its value, a run's width apart, so the two never
    // read as one.
    readonly property real labelsGap: Theme.space.xl
    // A number and its unit kept on one line, so "8 s" never leaves its s
    // alone on the next (VKCF3Z, 2026-11-03).
    function unbroken(words) {
        return String(words).replace(/(\d) (?=[A-Za-z\u00b5\u00b0%])/g, "$1\u00a0");
    }
    // Where the first line's box begins, down from the content's top.
    readonly property real firstLine: drawer.rowLine - Theme.border - drawer.pitch / 2

    // The time, read as it turns each second while the moment shows.
    SystemClock {
        id: clock
        enabled: drawer.moment && drawer.visible
        precision: SystemClock.Seconds
    }
    // The time's capitals, which are centred on Now's row.
    FontMetrics {
        id: timeFace
        font: timeText.font
    }

    Flickable {
        id: view
        width: parent.width
        // Ending at the last whole line that keeps its inset and a step
        // from the frame below, as the text does on every other side, so
        // nothing runs into the foot of the plate; Shift+Up and Down bring
        // up the rest (VKCF3Z, 2026-11-03).
        height: drawer.firstLine + Math.max(0, Math.floor((drawer.tall - 2 * Theme.border
            - Theme.inset - Theme.space.l - drawer.firstLine) / drawer.pitch)) * drawer.pitch
        clip: true
        interactive: false
        contentWidth: width
        // Never ending between two lines, so scrolled to its end its lines
        // are still the card's.
        readonly property real natural: drawer.moment ? momentShown.ends : flow.y + flow.height
        contentHeight: natural <= height ? natural
            : height + Math.ceil((natural - height) / drawer.pitch) * drawer.pitch

        Item {
            id: column
            // The letters an inset and a step from the frame on either side,
            // as the card's beside it are.
            x: Theme.inset + Theme.space.l
            width: view.width - 2 * x
            height: flow.y + flow.height + Theme.inset + Theme.space.l

            // When or where, in capitals over the plate, on the field's
            // centre: a label, set apart by case and tracking rather than
            // size alone, so it reads as one before the name.
            Text {
                id: eyebrowText
                width: parent.width
                y: drawer.fieldLine - Theme.border + drawer.eyebrowDropped - baselineOffset
                visible: drawer.eyebrow !== ""
                text: drawer.eyebrow
                textFormat: Text.PlainText
                elide: Text.ElideRight
                color: Theme.muted
                font.family: Theme.level.label.family
                font.pointSize: Theme.level.label.pointSize
                font.weight: Theme.level.label.weight
                font.capitalization: Theme.level.label.capitalization
                font.letterSpacing: Theme.level.label.letterSpacing
                font.features: Theme.level.label.features
            }

            // The moment: the time, its seconds, what needs Joel and the
            // facts behind it, each on a line of one of the card's rows.
            Item {
                id: momentShown
                width: parent.width
                visible: drawer.moment
                // Where its last line's box ends, for the view to hold.
                readonly property real ends: groupsShown.count === 0 ? sayText.y + drawer.pitch
                    : groupsShown.lastBottom

                // The time, the one large thing, at Display in the mono, as
                // the lock's clock is, since figures are the mono's: all one
                // width so they never jiggle, its capitals centred on Now's
                // row, the first.
                Text {
                    id: timeText
                    readonly property real base: (drawer.lineOf(0) ?? drawer.rowLine)
                        + timeFace.tightBoundingRect("H").height / 2
                    y: base - Theme.border - baselineOffset
                    text: Moment.clock(clock.date)
                    textFormat: Text.PlainText
                    color: Theme.strong
                    font.family: Theme.level.display.family
                    font.pointSize: Theme.level.display.pointSize
                    font.weight: Theme.level.display.weight
                    font.letterSpacing: Theme.level.display.letterSpacing
                    font.features: Theme.level.display.features
                }

                // Its seconds after it, quiet and counting, at Note on the
                // time's baseline: upright, since italic is only the voice.
                Text {
                    id: secondsText
                    x: timeText.x + timeText.contentWidth + Theme.space.s
                    width: parent.width - x
                    y: timeText.base - Theme.border - baselineOffset
                    text: String(drawer.shown?.seconds ?? "")
                        .replace("{seconds}", String(clock.date.getSeconds()))
                    textFormat: Text.PlainText
                    elide: Text.ElideRight
                    color: Theme.muted
                    font.family: Theme.level.note.family
                    font.pointSize: Theme.level.note.pointSize
                    font.features: { "tnum": 1 }
                }

                // What needs Joel, or that nothing does, in the voice, on
                // its row's baseline.
                Text {
                    id: sayText
                    readonly property real base: drawer.rowBase(drawer.momentLines.say ?? -1,
                        timeText.base)
                    width: parent.width
                    y: base - Theme.border - baselineOffset
                    visible: text !== ""
                    text: drawer.shown?.say ?? ""
                    textFormat: Text.PlainText
                    elide: Text.ElideRight
                    color: Theme.text
                    font.family: Theme.level.voice.family
                    font.italic: Theme.level.voice.italic
                    font.pointSize: Theme.level.voice.pointSize
                    font.variableAxes: Theme.level.voice.variableAxes
                }

                // Each group of facts from its row's line, on the pitch.
                Repeater {
                    id: groupsShown
                    model: drawer.groups
                    // Where the last group's last line's box ends.
                    readonly property real lastBottom: {
                        const group = count > 0 ? itemAt(count - 1) : null;
                        return group ? group.y + group.height : 0;
                    }

                    Column {
                        id: group
                        required property var modelData
                        required property int index
                        // Its first baseline: its row's, or one empty line
                        // below the group or the words before it.
                        readonly property real base: {
                            const before = index === 0 ? sayText.base
                                : groupsShown.itemAt(index - 1)?.lastBase ?? sayText.base;
                            return drawer.rowBase(modelData.line, before);
                        }
                        // Each fact's first baseline, down from its top.
                        readonly property var bases: Array.from({ length: facts.count },
                            (_, at) => {
                                const row = facts.itemAt(at);
                                return row ? row.y + drawer.pitch / 2 + drawer.dropped : 0;
                            })
                        // Its last fact's first baseline.
                        readonly property real lastBase: base + height - drawer.pitch

                        // Named, not its parent, which a Repeater's
                        // delegate has none of while it's made or let go.
                        width: momentShown.width
                        y: base - Theme.border - drawer.pitch / 2 - drawer.dropped

                        Repeater {
                            id: facts
                            model: group.modelData.facts

                            Item {
                                id: momentFact
                                required property var modelData
                                width: group.width
                                height: Math.max(1, factName.lineCount, figure.lineCount)
                                    * drawer.pitch

                                Text {
                                    id: factName
                                    width: drawer.labelsWide
                                    y: drawer.pitch / 2 + drawer.dropped - baselineOffset
                                    text: momentFact.modelData.label ?? ""
                                    textFormat: Text.PlainText
                                    wrapMode: Text.Wrap
                                    lineHeightMode: Text.FixedHeight
                                    lineHeight: drawer.pitch
                                    color: Theme.muted
                                    font.family: Theme.level.note.family
                                    font.pointSize: Theme.level.note.pointSize
                                }

                                Text {
                                    id: figure
                                    x: drawer.labelsWide + drawer.labelsGap
                                    width: parent.width - x
                                    y: drawer.pitch / 2 + drawer.dropped - baselineOffset
                                    text: drawer.qualified(momentFact.modelData)
                                    textFormat: Text.StyledText
                                    wrapMode: Text.Wrap
                                    lineHeightMode: Text.FixedHeight
                                    lineHeight: drawer.pitch
                                    color: Theme.text
                                    font.family: Theme.level.figure.family
                                    font.pointSize: Theme.level.figure.pointSize
                                    font.features: Theme.level.figure.features
                                }
                            }
                        }
                    }
                }
            }

            // The title, then its facts, words and picture, each a group
            // of whole lines one empty line apart, from the first row's
            // line on.
            Column {
                id: flow
                y: drawer.firstLine
                width: parent.width
                spacing: drawer.pitch
                visible: !drawer.moment

                // The thing's name at Title, sized by capitals as every
                // serif step is, on the first row's baseline beside it,
                // whatever its own line box would put it on. Too large for
                // the pitch, a name that wraps takes two of it a line.
                Item {
                    width: parent.width
                    height: (2 * titleText.lineCount - 1) * drawer.pitch
                    visible: drawer.title !== ""

                    Text {
                        id: titleText
                        width: parent.width
                        y: drawer.pitch / 2 + drawer.dropped - baselineOffset
                        // A name with no space, as a server's or a path,
                        // may break after a dot or a slash rather than
                        // mid-word, where it's as wide as the Title allows.
                        text: drawer.title.replace(/([./_])(?=\S)/g, "$1\u200b")
                        textFormat: Text.PlainText
                        wrapMode: Text.Wrap
                        lineHeightMode: Text.FixedHeight
                        lineHeight: 2 * drawer.pitch
                        color: Theme.level.title.colour
                        font.family: Theme.level.title.family
                        font.pointSize: Theme.level.title.pointSize
                        font.letterSpacing: Theme.level.title.letterSpacing
                        font.variableAxes: Theme.level.title.variableAxes
                    }
                }

                // Its preview, right under its name (VKCF3Z, 2026-11-03): a
                // picture in its own shape, as wide as the plate at most and
                // never so tall it pushes the facts after it out of sight, or
                // lines in the mono at body size, each whole or cut short,
                // a folder's items quiet. It takes whole lines of the pitch,
                // so everything after it stays on the card's.
                Item {
                    id: previewBox
                    readonly property string picture: drawer.preview?.picture ?? ""
                    // As many lines as the room holds, so the facts stay
                    // in sight under them.
                    readonly property var lines: (drawer.preview?.lines ?? [])
                        .slice(0, Math.floor(room / drawer.pitch))
                    // The room it may take: what's left of the view once
                    // the name and the facts have theirs, a line apart.
                    readonly property real room: Math.max(4 * drawer.pitch, view.height
                        - drawer.firstLine - (titleText.visible ? 2 * titleText.lineCount * drawer.pitch
                        : 0) - (factsShown.visible ? factsShown.height
                        + drawer.pitch : 0) - drawer.pitch)

                    width: parent.width
                    visible: picture !== "" || lines.length > 0
                    height: Math.ceil((picture !== "" ? previewFrame.height
                        : lines.length * drawer.pitch) / drawer.pitch) * drawer.pitch

                    Rectangle {
                        id: previewFrame
                        readonly property real across: drawer.preview?.across > 0
                            ? drawer.preview.across : parent.width
                        readonly property real down: drawer.preview?.down > 0
                            ? drawer.preview.down : across
                        readonly property real shrunk: Math.min(1, parent.width / across,
                            Math.floor(previewBox.room / drawer.pitch) * drawer.pitch / down)

                        visible: previewBox.picture !== ""
                        width: Math.round(across * shrunk)
                        height: Math.round(down * shrunk)
                        radius: Theme.inner
                        color: "transparent"

                        // It comes in once it's drawn, over the short
                        // duration, rather than as a box waiting for it.
                        Image {
                            id: previewDrawn
                            anchors.fill: parent
                            source: previewBox.picture === "" ? "" : `file://${previewBox.picture}`
                            asynchronous: true
                            fillMode: Image.PreserveAspectFit
                            sourceSize.width: 2 * width
                            sourceSize.height: 2 * height
                            opacity: status === Image.Ready ? 1 : 0
                            layer.enabled: true
                            layer.effect: MultiEffect {
                                maskEnabled: true
                                maskSource: previewCorners
                            }
                            Behavior on opacity {
                                NumberAnimation {
                                    duration: Theme.motion.short
                                    easing.type: Easing.BezierSpline
                                    easing.bezierCurve: Theme.motion.curve
                                }
                            }
                            onStatusChanged: {
                                if (status === Image.Ready)
                                    Report.say(`previewed ${drawer.title}`);
                            }
                        }

                        Rectangle {
                            id: previewCorners
                            anchors.fill: parent
                            radius: Theme.inner
                            visible: false
                            layer.enabled: true
                        }
                    }

                    Item {
                        id: previewLines
                        width: parent.width
                        height: parent.height
                        visible: previewBox.picture === "" && previewBox.lines.length > 0

                        Repeater {
                            id: previewRows
                            model: previewBox.lines

                            Text {
                                required property string modelData
                                required property int index

                                width: previewLines.width
                                y: index * drawer.pitch + drawer.pitch / 2 + drawer.dropped
                                    - baselineOffset
                                text: modelData
                                textFormat: Text.PlainText
                                elide: Text.ElideRight
                                color: drawer.preview?.quiet === true ? Theme.muted : Theme.text
                                font.family: Theme.level.note.family
                                font.pointSize: Theme.level.note.pointSize
                            }
                        }
                    }
                }

                Column {
                    id: factsShown
                    width: parent.width
                    visible: drawer.facts.length > 0

                    Repeater {
                        id: factRows
                        model: drawer.facts

                        Item {
                            id: fact
                            required property var modelData
                            required property int index
                            // A heading over the facts after it, as the
                            // keys' list names each mode's, set as the
                            // eyebrow is and an empty line below any
                            // fact before it.
                            readonly property bool heading: modelData.heading !== undefined
                            readonly property real headingRoom: heading && index > 0 ? drawer.pitch
                                : 0
                            readonly property bool known: modelData.value !== null
                                && modelData.value !== undefined
                            // Its first baseline, down from its top.
                            readonly property real base: drawer.pitch / 2 + drawer.dropped
                                + headingRoom
                            // A value that won't sit beside its name in two
                            // lines takes the plate's whole width on the
                            // line under it, so it never reads a word a line
                            // (VKCF3Z, 2026-11-04).
                            readonly property bool under: beside.lineCount > 2
                            readonly property real valueFrom: under ? 0
                                : drawer.labelsWide + drawer.labelsGap

                            width: parent.width
                            height: heading ? headingRoom + drawer.pitch
                                : (under ? label.lineCount + value.lineCount
                                    : Math.max(1, label.lineCount, value.lineCount)) * drawer.pitch

                            Text {
                                width: parent.width
                                y: fact.base - baselineOffset
                                visible: fact.heading
                                text: fact.modelData.heading ?? ""
                                textFormat: Text.PlainText
                                elide: Text.ElideRight
                                color: Theme.muted
                                font.family: Theme.level.label.family
                                font.pointSize: Theme.level.label.pointSize
                                font.weight: Theme.level.label.weight
                                font.capitalization: Theme.level.label.capitalization
                                font.letterSpacing: Theme.level.label.letterSpacing
                                font.features: Theme.level.label.features
                            }

                            // The value as it would wrap beside its name,
                            // measured and never drawn.
                            Text {
                                id: beside
                                visible: false
                                width: fact.width - drawer.labelsWide - drawer.labelsGap
                                text: value.text
                                textFormat: Text.PlainText
                                wrapMode: Text.Wrap
                                font: value.font
                            }

                            Text {
                                id: label
                                visible: !fact.heading
                                width: drawer.labelsWide
                                y: fact.base - baselineOffset
                                text: fact.modelData.label ?? ""
                                textFormat: Text.PlainText
                                wrapMode: Text.Wrap
                                lineHeightMode: Text.FixedHeight
                                lineHeight: drawer.pitch
                                color: Theme.muted
                                font.family: Theme.level.note.family
                                font.pointSize: Theme.level.note.pointSize
                            }

                            // Its figure, or a placeholder while it's
                            // asked, on the one edge every value starts
                            // at, a long one breaking onto the pitch.
                            Text {
                                id: value
                                visible: !fact.heading
                                x: fact.valueFrom
                                width: parent.width - x
                                y: fact.base - baselineOffset
                                    + (fact.under ? label.lineCount * drawer.pitch : 0)
                                text: fact.known ? drawer.unbroken(fact.modelData.value) : "\u2026"
                                textFormat: Text.PlainText
                                wrapMode: Text.Wrap
                                lineHeightMode: Text.FixedHeight
                                lineHeight: drawer.pitch
                                color: fact.known ? Theme.text : Theme.muted
                                font.family: Theme.level.figure.family
                                font.pointSize: Theme.level.figure.pointSize
                                font.features: Theme.level.figure.features
                            }
                        }
                    }
                }

                // Its words, plain unless the row made them and says
                // they're styled: another program's are never read as
                // markup. They keep the pitch, each line on the card's.
                Item {
                    id: wordsBox
                    width: parent.width
                    height: bodyText.lineCount * drawer.pitch
                    visible: drawer.body !== ""

                    Text {
                        id: bodyText
                        width: parent.width
                        y: drawer.pitch / 2 + drawer.dropped - baselineOffset
                        text: drawer.body
                        textFormat: drawer.shown?.styled === true ? Text.StyledText
                            : Text.PlainText
                        wrapMode: Text.Wrap
                        lineHeightMode: Text.FixedHeight
                        lineHeight: drawer.pitch
                        color: Theme.text
                        // Prose, such as a notice's, at Words, as its card
                        // reads it and the reader sets its text. The rest,
                        // lines another program wrote, at Figure.
                        readonly property var level: drawer.prose ? Theme.level.words
                            : Theme.level.figure
                        font.family: bodyText.level.family
                        font.pointSize: bodyText.level.pointSize
                        font.variableAxes: bodyText.level.variableAxes
                    }
                }

                // A picture in its own shape, as wide as the drawer at
                // most, its corners a part's, the raised colour until it's
                // drawn. One whose size isn't given is as wide as the
                // drawer, and square: never its own width, which is worked
                // out from it.
                Rectangle {
                    id: frame
                    readonly property real across: drawer.shown?.across > 0
                        ? drawer.shown.across : parent.width
                    readonly property real down: drawer.shown?.down > 0
                        ? drawer.shown.down : across
                    readonly property real shrunk: Math.min(1, parent.width / across)

                    visible: drawer.picture !== ""
                    width: Math.round(across * shrunk)
                    height: Math.round(down * shrunk)
                    radius: Theme.inner
                    color: drawn.status === Image.Ready ? "transparent" : Theme.raised

                    Image {
                        id: drawn
                        anchors.fill: parent
                        source: drawer.picture === "" ? "" : `file://${drawer.picture}`
                        asynchronous: true
                        fillMode: Image.PreserveAspectFit
                        sourceSize.width: 2 * width
                        sourceSize.height: 2 * height
                        layer.enabled: true
                        layer.effect: MultiEffect {
                            maskEnabled: true
                            maskSource: corners
                        }
                        onStatusChanged: {
                            if (status === Image.Ready)
                                Report.say(`showed ${drawer.title}`);
                        }
                    }

                    Rectangle {
                        id: corners
                        anchors.fill: parent
                        radius: Theme.inner
                        visible: false
                        layer.enabled: true
                    }
                }
            }
        }
    }
}
