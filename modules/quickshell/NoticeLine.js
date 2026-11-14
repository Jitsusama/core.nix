// What the arrival line says (NoticeLine.qml): which notice it shows,
// whether it's up, and how many more wait, worked out apart from the
// surface so tests/notice-line.qml can hold it in Qt's own engine. It
// orders nothing: the notices daemon serves which one is in front, the
// first critical one and how many wait (modules/notices' front.rs), since
// the daemons are the data layer (Joel, 2026-11-12), and the line only
// picks between the two by what the shell alone knows.
.pragma library

// The notice the line shows, by id, or -1: the one in front, or while
// notices are held (Quiet.qml) only the first critical one, as only a
// critical card shows then. One read or withdrawn leaves the front at
// once, since it no longer asks anything of him (Joel, 2026-11-12: what's
// no longer applicable should disappear).
function chosen(front, held) {
    const id = held ? front?.critical : front?.id;
    return typeof id === "number" ? id : -1;
}

// Whether the line is up with a notice chosen. It steps aside while the
// notices are gathered where the launcher's keys reach them, so each is in
// one place; and under the talk line, which sits in the same place, only a
// critical one shows, since he's saying something.
function up(chosenId, gathered, talking, critical) {
    if (chosenId < 0 || gathered) {
        return false;
    }
    return !talking || critical;
}

// What the line shows: "notice", "playing" or nothing. What's playing
// shows for its moment (Music.qml's arrival) over a notice that can wait,
// since a media key Joel just pressed put it there, but never over a
// critical one, nor where the notice would step aside: in the launcher's
// notices or under the talk line.
function shows({ noticeUp, critical, playing, gathered, talking }) {
    if (noticeUp && critical) {
        return "notice";
    }
    if (playing && !gathered && !talking) {
        return "playing";
    }
    return noticeUp ? "notice" : "";
}

// Plain text as styled text, which a notice's words are, so what's playing
// can stand where they do with an artist's name read as written.
function escaped(text) {
    return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

// How long a notice stays on the line before it rests, in milliseconds:
// what its sender asked, or five seconds, ten for one offering something
// to do. Quickshell's documents call the sender's timeout seconds, but it
// passes on what the sender said, which is milliseconds.
function lasts(expireTimeout, actions) {
    if (expireTimeout > 0) {
        return expireTimeout;
    }
    return actions > 0 ? 10000 : 5000;
}

// How many others wait behind the one shown, which the line counts at its
// far end, and Super+N reaches them all: every one waiting, the one shown
// aside.
function more(front, chosenId) {
    const waiting = front?.waiting ?? 0;
    return Math.max(0, chosenId < 0 ? waiting : waiting - 1);
}
