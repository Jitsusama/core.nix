// The launcher's drawer: what the selected row has to show beyond its one
// line, out of the card's right side rather than below its rows (Joel,
// 2026-10-28: "having it come out of the bottom feels wrong, i think it
// should come out of the right side like a drawer, making sure it never
// passes the bound of the screen"). It's the card continued, a floating
// surface like it, as tall as the card and top aligned with it, so the
// card never changes size for what's selected.
//
// It's set as a plate, a small page about the selected thing (look 3's
// piece 3): an eyebrow saying when or where, the thing's name in the
// voice's face, its facts as a name and a figure, its words, then its
// picture. Every line of it lies on one of the card's lines, so the two
// read as one page: the eyebrow on the field's centre, the name on the
// first row's baseline, and the rest on a pitch of half a row from the
// second row's, one empty line between each group. A row gives it as
// `drawer: { eyebrow, title, facts: [{ label, value }], body, styled,
// picture, across, down }`, the body plain unless it says it's styled, a
// fact whose value isn't known yet null, shown as a fixed placeholder
// until it is. Shift+Up and Shift+Down scroll it a few lines at a time,
// as the launcher's foot says while it holds more than fits
// (docs/design.md, the drawer).
pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Effects

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

    readonly property string eyebrow: shown ? shown.eyebrow ?? "" : ""
    readonly property string title: shown ? shown.title ?? "" : ""
    readonly property var facts: shown ? (shown.facts ?? []).filter(fact => fact) : []
    readonly property string body: shown ? shown.body ?? "" : ""
    readonly property string picture: shown ? shown.picture ?? "" : ""
    // Whether it holds more than fits, so the foot says how to scroll it.
    readonly property bool more: view.contentHeight > view.height + 1
    // How far it's scrolled, for the launcher to report.
    readonly property real scrolled: view.contentY

    // The plate's pitch: half a row, so its lines fall on each row's and
    // halfway between two.
    readonly property real pitch: Theme.space.xl
    // How far below a line's centre a line of the mono at the launcher's
    // size has its baseline, as a row centred on that line does, and the
    // same at body size for the eyebrow. Every line here is set on such a
    // baseline, whatever its own face, since the faces' line boxes differ.
    readonly property real dropped: monoRuler.y + monoRuler.baselineOffset
    readonly property real eyebrowDropped: eyebrowRuler.y + eyebrowRuler.baselineOffset

    // Where its lines' baselines are, down from its top, for the launcher
    // to report: the eyebrow's and the middle of its line, the title's,
    // each fact's first line's and the words' first line's.
    readonly property real contentTop: Theme.border - view.contentY
    readonly property var baselines: ({
        eyebrow: eyebrowText.visible ? contentTop + eyebrowText.y + eyebrowText.baselineOffset
            : null,
        eyebrowMiddle: eyebrowText.visible ? contentTop + eyebrowText.y
            + eyebrowText.height / 2 : null,
        words: wordsBox.visible ? contentTop + flow.y + wordsBox.y + bodyText.y
            + bodyText.baselineOffset : null,
        title: titleText.visible ? contentTop + flow.y + titleText.y + titleText.baselineOffset
            : null,
        facts: factsShown.visible ? Array.from({ length: factRows.count }, (_, at) => {
            const row = factRows.itemAt(at);
            return row ? contentTop + flow.y + factsShown.y + row.y + row.base : null;
        }) : []
    })

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
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
    }
    Text {
        id: eyebrowRuler
        y: -height / 2
        visible: false
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.body
    }

    // The widest of its facts' names, which their values start beyond, so
    // the values share one edge.
    FontMetrics {
        id: factFace
        font.family: Theme.mono.family
        font.pointSize: Theme.mono.large
    }
    readonly property real labelsWide: Math.min(column.width * 0.4,
        Math.ceil(Math.max(0, ...facts.map(fact => factFace.advanceWidth(fact.label ?? "")))))

    Flickable {
        id: view
        width: parent.width
        height: drawer.tall - 2 * Theme.border
        clip: true
        interactive: false
        contentWidth: width
        // Never ending between two lines, so scrolled to its end its lines
        // are still the card's.
        readonly property real natural: column.height
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
                font.family: Theme.mono.family
                font.pointSize: Theme.mono.body
                font.weight: Font.Medium
                font.capitalization: Font.AllUppercase
                font.letterSpacing: Theme.mono.body * 4 / 3 * 0.16
                font.features: { "case": 1 }
            }

            // The title, then its facts, words and picture, each a group
            // of whole lines one empty line apart, from the first row's
            // line on.
            Column {
                id: flow
                y: drawer.rowLine - Theme.border - drawer.pitch / 2
                width: parent.width
                spacing: drawer.pitch

                // The thing's name, in the voice's face upright and a step
                // up, sized by capitals as every serif step is, on the
                // first row's baseline beside it, whatever its own line box
                // would put it on.
                Item {
                    width: parent.width
                    height: titleText.lineCount * drawer.pitch
                    visible: drawer.title !== ""

                    Text {
                        id: titleText
                        width: parent.width
                        y: drawer.pitch / 2 + drawer.dropped - baselineOffset
                        text: drawer.title
                        textFormat: Text.PlainText
                        wrapMode: Text.Wrap
                        lineHeightMode: Text.FixedHeight
                        lineHeight: drawer.pitch
                        color: Theme.strong
                        font.family: Theme.serif.family
                        font.pointSize: Theme.serif.title
                        font.variableAxes: Object.assign({}, Theme.serif.axes,
                            { wght: 500, opsz: 72 })
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
                            readonly property bool known: modelData.value !== null
                                && modelData.value !== undefined
                            // Its first baseline, down from its top.
                            readonly property real base: drawer.pitch / 2 + drawer.dropped

                            width: parent.width
                            height: Math.max(1, value.lineCount) * drawer.pitch

                            Text {
                                id: label
                                width: drawer.labelsWide
                                y: fact.base - baselineOffset
                                text: fact.modelData.label ?? ""
                                textFormat: Text.PlainText
                                elide: Text.ElideRight
                                color: Theme.muted
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.large
                            }

                            // Its figure, or a placeholder while it's
                            // asked, on the one edge every value starts
                            // at, a long one breaking onto the pitch.
                            Text {
                                id: value
                                x: drawer.labelsWide + Theme.space.l
                                width: parent.width - x
                                y: fact.base - baselineOffset
                                text: fact.known ? String(fact.modelData.value) : "\u2026"
                                textFormat: Text.PlainText
                                wrapMode: Text.Wrap
                                lineHeightMode: Text.FixedHeight
                                lineHeight: drawer.pitch
                                color: fact.known ? Theme.text : Theme.muted
                                font.family: Theme.mono.family
                                font.pointSize: Theme.mono.large
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
                        font.family: Theme.mono.family
                        font.pointSize: Theme.mono.large
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
