pragma Singleton
// For You's Slack rows as Slack's daemon serves them ready: each message
// wanting Joel, done ones too, newest first (src/message_rows.rs), and
// each conversation unread, in Slack's order (src/rows.rs), held once
// for every surface showing the piece, as SlackUnreads holds the rest.
// Each is made the one row every surface shows of it (SlackMessage's and
// SlackConversation's `served`) with the `thing` of the surface asking,
// Now's, as a piece's rows are (Piece.qml's rows). How many there are,
// who and when, and how Slack stands stay SlackUnreads', until the daemon
// serves the piece's own row too.
import qs
import Quickshell
import QtQuick
import "launcher" as Launcher
import "SlackUnreads.js" as Unreads

Singleton {
    id: root

    // What makes each row served, the `thing` of the surface last asking,
    // as ReadyRows asks a place's provider to (its `maker`).
    QtObject {
        id: making

        property var made: null

        function member(fields) {
            return made(fields);
        }

        function thing(fields) {
            return made(fields);
        }
    }

    // A message's time beside it reads as of the minute, and nothing asks
    // for these rows but a surface listing them, so they're said again as
    // each minute turns whether or not one is drawn.
    Launcher.ReadyRows {
        id: messages

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/slack.sock`
        place: "slack-messages"
        beside: ["drawer"]
        maker: making
        make: (row, key) => SlackMessage.served(messages, row, key)
        drawn: true
    }
    Launcher.ReadyRows {
        id: conversations

        socket: `${Quickshell.env("XDG_RUNTIME_DIR")}/daemons/slack.sock`
        place: "slack-unread"
        maker: making
        make: (row, key) => SlackConversation.served(conversations, row, key)
    }

    // The rows of `ready` whose served row `keeps` holds, in its order.
    function kept(ready, keeps) {
        void ready.placings;
        return ready.keys.filter(key => keeps(ready.served(key) ?? {}))
            .map(key => ready.rowAt(key)).filter(row => row !== null);
    }

    // For You's Slack rows, made with `thing`: while the daemon holds every
    // message, each still to read and then Threads, whose replies it never
    // holds; else each conversation wanting Joel, a channel merely unread
    // not among them (SlackUnreads.js's wants).
    function forYou(thing) {
        making.made = thing;
        const whole = SlackUnreads.messages !== null;
        const talks = kept(conversations,
            one => whole ? one.kind === "thread" : Unreads.wants(one));
        return whole ? kept(messages, one => !Unreads.done(one.held ?? {})).concat(talks)
            : talks;
    }

    // Each message done with, read or gone from Slack, as For You's History
    // takes them (Lately.js's done): who, how and when, and its row; null
    // while the daemon doesn't hold every message.
    readonly property var finished: {
        void messages.placings;
        if (SlackUnreads.messages === null)
            return null;
        return messages.keys.map(key => messages.served(key)).filter(row =>
            row !== null && Unreads.done(row.held ?? {})).map(row => ({
                who: row.name,
                done: row.held.done,
                row: thing => {
                    making.made = thing;
                    return messages.rowAt(row.key);
                }
            }));
    }
}
