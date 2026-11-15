// Polkit's prompt as a card (Polkit.qml): who asks in the mono and whose
// password a strong span, then why or what went wrong, then the padlock
// beside the dots, then what the keys do. It's apart from the agent so its
// pictures can be drawn with nothing asking polkit.
pragma ComponentBehavior: Bound
import QtQuick
import "Polkit.js" as Said

Floating {
    id: card

    // Who asks, as Polkit.js says it, and the account whose password it is.
    property string asker
    property string who
    // What the asking process says of itself, as asker-says answers it;
    // the card words it with Polkit.js's says, none when it says nothing.
    property string said
    // Why, or what went wrong once it's failed, in the alert.
    property string reason
    property bool failed: false
    // Whether what's typed shows as dots, as polkit asks for a password.
    property bool secret: true

    signal submitted(string text)
    signal refused

    keys: true
    urgent: failed

    // The reason's face, to measure what the asker says as it's drawn.
    TextMeasure {
        id: saidMeasure
        font.family: head.reasonLevel.family
        font.pointSize: head.reasonLevel.pointSize
        font.weight: head.reasonLevel.weight
        font.variableAxes: head.reasonLevel.variableAxes
        font.features: head.reasonLevel.features
    }
    FontMetrics {
        id: saidFace
        font: saidMeasure.font
    }

    Column {
        width: parent.width

        // Who asks in the mono, as a program is named, and whose password a
        // strong span; then why, or what went wrong. The command breaks
        // only at its spaces (Said.joined), and what it needs holds together
        // with no-break spaces, so no word of it is left alone on a line
        // (VKCF3Z, 2026-11-15).
        PromptHead {
            id: head
            width: parent.width
            readonly property string asker: `<span style="font-family:`
                + `'${Theme.mono.family}'">${Said.plain(Said.joined(card.asker))}</span>`
            readonly property string owner: `<span style="color:${Theme.strong};`
                + `font-weight:${Theme.level.words.spans.strong}">${card.who}</span>`
            asked: `${asker} needs\u00a0${owner}'s\u00a0password`
            askedOpens: card.asker.charAt(0)
            askedOpensIn: Theme.mono.family
            // The room a line of the reason has, from the inset at each
            // side and its first letter's bearing, as the head sets it.
            readonly property real saidRoom: width - 2 * inset
                - saidFace.tightBoundingRect("I").x
            // What it says of itself comes first, under the command it's
            // said of, then why, as lines of one saying; it gives way to
            // stay one line, measured as it's drawn (VKCF3Z, 2026-11-15).
            // It reads the face, so it's worded again when that changes, as
            // TextMeasure.qml asks.
            readonly property string saidLine: Said.says(card.said,
                line => saidMeasure.font && saidMeasure.widthOf(line) <= saidRoom)
            reason: [saidLine, card.reason].filter(line => line !== "").join("\n")
            reasonColour: card.failed ? Theme.alert : Theme.level.words.colour
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
                secret: card.secret
                onAccepted: {
                    card.submitted(text);
                    clear();
                }
                onCancelled: {
                    card.refused();
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
