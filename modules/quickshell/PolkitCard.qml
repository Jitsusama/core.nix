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
    // Why, or what went wrong once it's failed, in the alert.
    property string reason
    property bool failed: false
    // Whether what's typed shows as dots, as polkit asks for a password.
    property bool secret: true

    signal submitted(string text)
    signal refused

    keys: true
    urgent: failed

    Column {
        width: parent.width

        // Who asks in the mono, as a program is named, and whose password a
        // strong span; then why, or what went wrong. The command breaks
        // only at its spaces (Said.joined), and what it needs holds together
        // with no-break spaces, so no word of it is left alone on a line
        // (VKCF3Z, 2026-11-15).
        PromptHead {
            width: parent.width
            readonly property string asker: `<span style="font-family:`
                + `'${Theme.mono.family}'">${Said.plain(Said.joined(card.asker))}</span>`
            readonly property string owner: `<span style="color:${Theme.strong};`
                + `font-weight:${Theme.level.words.spans.strong}">${card.who}</span>`
            asked: `${asker} needs\u00a0${owner}'s\u00a0password`
            askedOpens: card.asker.charAt(0)
            askedOpensIn: Theme.mono.family
            reason: card.reason
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
