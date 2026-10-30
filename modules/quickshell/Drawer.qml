// The launcher's drawer: what the selected row has to show beyond its one
// line, out of the card's right side rather than below its rows (Joel,
// 2026-10-28: "having it come out of the bottom feels wrong, i think it
// should come out of the right side like a drawer, making sure it never
// passes the bound of the screen"). It's the card continued, a floating
// surface like it, as tall as the card and top aligned with it, so the
// card never changes size for what's selected: the row's title, its facts
// as a name and a figure, its words, then its picture. A row gives it as
// `drawer: { title, facts: [{ label, value }], body, styled, picture,
// across, down }`, the body plain unless it says it's styled, a fact whose
// value isn't known yet null, shown as a fixed placeholder until it is.
// Ctrl+Up and Ctrl+Down scroll it, as the launcher's foot says while it
// holds more than fits (docs/design.md, the drawer).
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
    // How tall it is, the card's height.
    property real tall: 0

    readonly property string title: shown ? shown.title ?? "" : ""
    readonly property var facts: shown ? (shown.facts ?? []).filter(fact => fact) : []
    readonly property string body: shown ? shown.body ?? "" : ""
    readonly property string picture: shown ? shown.picture ?? "" : ""
    // Whether it holds more than fits, so the foot says how to scroll it.
    readonly property bool more: view.contentHeight > view.height + 1
    // How far it's scrolled, for the launcher to report.
    readonly property real scrolled: view.contentY

    // Moves what it shows a few lines up or down, never past either end.
    function scroll(lines) {
        const line = titleText.implicitHeight;
        view.contentY = Math.max(0, Math.min(view.contentHeight - view.height,
            view.contentY + lines * 3 * line));
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

    Flickable {
        id: view
        width: parent.width
        height: drawer.tall - 2 * Theme.border
        clip: true
        interactive: false
        contentWidth: width
        contentHeight: column.height

        Column {
            id: column
            x: Theme.space.l
            width: view.width - 2 * Theme.space.l
            topPadding: Theme.space.l
            bottomPadding: Theme.space.l
            spacing: Theme.space.m

            Text {
                id: titleText
                width: parent.width
                visible: drawer.title !== ""
                text: drawer.title
                textFormat: Text.PlainText
                wrapMode: Text.Wrap
                color: Theme.strong
                font.family: Theme.mono.family
                font.pointSize: Theme.mono.large
                font.weight: Font.Medium
            }

            Column {
                width: parent.width
                visible: drawer.facts.length > 0
                spacing: Theme.space.xs

                Repeater {
                    model: drawer.facts

                    Item {
                        id: fact
                        required property var modelData
                        readonly property bool known: modelData.value !== null
                            && modelData.value !== undefined

                        width: parent.width
                        height: Math.max(label.implicitHeight, value.implicitHeight)

                        Text {
                            id: label
                            width: Math.min(implicitWidth, parent.width * 0.4)
                            text: fact.modelData.label ?? ""
                            textFormat: Text.PlainText
                            elide: Text.ElideRight
                            color: Theme.muted
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.large
                        }

                        // Its figure, or a placeholder the width of one
                        // while it's asked, so nothing moves when it comes.
                        Text {
                            id: value
                            x: label.width + Theme.space.m
                            width: parent.width - x
                            text: fact.known ? String(fact.modelData.value) : "\u2026"
                            textFormat: Text.PlainText
                            wrapMode: Text.Wrap
                            horizontalAlignment: Text.AlignRight
                            color: fact.known ? Theme.text : Theme.muted
                            font.family: Theme.mono.family
                            font.pointSize: Theme.mono.large
                        }
                    }
                }
            }

            // Its words, plain unless the row made them and says they're
            // styled: another program's are never read as markup.
            Text {
                width: parent.width
                visible: drawer.body !== ""
                text: drawer.body
                textFormat: drawer.shown?.styled === true ? Text.StyledText : Text.PlainText
                wrapMode: Text.Wrap
                color: Theme.text
                font.family: Theme.mono.family
                font.pointSize: Theme.mono.large
            }

            // A picture in its own shape, as wide as the drawer at most,
            // its corners a part's, the raised colour until it's drawn. One
            // whose size isn't given is as wide as the drawer, and square:
            // never its own width, which is worked out from it.
            Rectangle {
                id: frame
                readonly property real across: drawer.shown?.across > 0
                    ? drawer.shown.across : parent.width
                readonly property real down: drawer.shown?.down > 0 ? drawer.shown.down : across
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
