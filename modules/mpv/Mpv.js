// What mpv's provider in the launcher works out (MpvProgram.qml), kept apart
// from Quickshell's types so tests/mpv-program.qml runs it in Qt's own
// engine: whether what was copied is one address mpv plays, how far into a
// file mpv stopped, said in words, the files mpv-stopped lists (stopped.sh),
// which of them were watched whole, what each one's row says beside it and
// its drawer's plate, and the commands that play each. A file's size and
// when it was left are said as the launcher says them of any file
// (FileFacts.js), so a file reads alike in mpv's Recent and the chooser.
.pragma library
.import "../FileFacts.js" as FileFacts

// The sites whose pages are something to watch or hear, by host, each with
// the paths that are one, its query after it, so a site's front page and
// the rest of its pages aren't offered. mpv plays them through yt-dlp,
// which its wrapper carries, but only these say what plays: nothing asks
// yt-dlp or the network.
const SITES = {
    "youtube.com":
        /^\/(?:watch\?(?:\S*&)?v=|playlist\?(?:\S*&)?list=|(?:shorts|live|embed)\/)[\w-]+/,
    "music.youtube.com": /^\/(?:watch\?(?:\S*&)?v=|playlist\?(?:\S*&)?list=)[\w-]+/,
    "youtu.be": /^\/[\w-]+(?:\?|$)/,
    "vimeo.com": /^\/(?:[\w-]+\/)*\d+(?:\?|$)/,
    "twitch.tv": /^\/(?:videos\/\d+|\w+\/clip\/[\w-]+|\w+)(?:\?|$)/,
    "clips.twitch.tv": /^\/[\w-]+(?:\?|$)/,
    "dailymotion.com": /^\/video\/\w+(?:\?|$)/,
    "dai.ly": /^\/\w+(?:\?|$)/,
    "streamable.com": /^\/\w+(?:\?|$)/,
    "soundcloud.com": /^\/[\w-]+\/[\w-]+(?:\?|$)/,
    "tiktok.com": /^\/@[\w.-]+\/video\/\d+(?:\?|$)/
};

// A file whose name says it's to watch or hear, as a server hands it over,
// or a stream's playlist. A .ts is left out, as it's TypeScript as often
// as it's a stream.
const MEDIA =
    /\.(?:mp4|m4v|mkv|webm|mov|avi|ogv|mpe?g|flv|3gp|mp3|m4a|aac|ogg|oga|opus|flac|wav|m3u8|mpd)$/i;

// An address as it's copied: its host, then its port, path, query and
// fragment, with no name or password before the host.
const ADDRESS = /^https?:\/\/([^/?#\s@:]+)(?::\d+)?(\/[^?#\s]*)?(\?[^#\s]*)?(?:#\S*)?$/i;

// What comes before a host's site but doesn't name it: "m.youtube.com"
// is youtube.com.
const LEADING = /^(?:www|m)\./;

const MINUTE = 60;
const HOUR = 3600;
const MILLISECONDS = 1000;

// How near its end, in seconds, a file mpv left counts as watched whole:
// mpv quit on a file held on its last frame (keep-open) keeps a time a
// frame or so short of its length, and stopped.sh reads the rest as part
// watched by the same second.
const END = 1;

// What mpv kept that the drawer says first, in this order, as Joel looks
// for them: how fast and how loud it played, and its audio and subtitle
// tracks. Any other option mpv kept follows them, in mpv's own order.
const FIRST = ["speed", "volume", "aid", "sid"];

// A figure as mpv writes an option's, "1.500000", and the length mpv may
// put before a value it quotes, "%3%no,".
const FIGURE = /^-?\d+(?:\.\d+)?$/;
const QUOTED = /^%\d+%/;
// The most decimals a figure mpv kept is said with.
const DECIMALS = 3;
// The options mpv keeps in seconds, each said with its unit and to the
// millisecond, "0.500 s", since a bare figure names no unit (VKCF3Z,
// 2026-11-08).
const IN_SECONDS = ["sub-delay", "secondary-sub-delay", "audio-delay"];

// An address's host, as its site is known by.
function siteOf(host) {
    return host.toLowerCase().replace(LEADING, "");
}

// The address mpv plays in what was copied, or "" when it holds anything
// else: more than one address, words around one, or an address of a page
// with nothing to play. The shell's watch of the clipboard (Copied.qml)
// already gives one web address alone as its `link`, which this narrows to
// the pages and files mpv plays.
function playable(copied) {
    const address = String(copied ?? "").trim();
    const parts = ADDRESS.exec(address);
    if (!parts) {
        return "";
    }
    const path = parts[2] ?? "/";
    const site = SITES[siteOf(parts[1])];
    if (site?.test(path + (parts[3] ?? "")) || MEDIA.test(path)) {
        return address;
    }
    return "";
}

// The site an address is on, as it's said beside it: "youtube.com".
function hostOf(address) {
    const parts = ADDRESS.exec(String(address ?? "").trim());
    return parts ? siteOf(parts[1]) : "";
}

// A time into a file as mpv's own clock says it, in whole seconds:
// "12:04", and past an hour "1:02:03".
function clock(seconds) {
    const counted = Math.max(0, Math.floor(Number(seconds) || 0));
    const hours = Math.floor(counted / HOUR);
    const minutes = Math.floor((counted % HOUR) / MINUTE);
    const rest = String(counted % MINUTE).padStart(2, "0");
    const said = hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}` : String(minutes);
    return `${said}:${rest}`;
}

// What mpv kept of a file besides where it stopped, each `option=value`
// as mpv wrote it: { option, value } each, an option split from its value
// at the first =, since a filter's value may hold one of its own.
function options(parts) {
    const read = [];
    for (const part of parts) {
        const at = part.indexOf("=");
        if (at > 0) {
            read.push({ option: part.slice(0, at), value: part.slice(at + 1) });
        }
    }
    return read;
}

// The files mpv-stopped printed, a line each: the file's whole path, where
// it stopped and how long it is, both in seconds, when mpv left it, in
// seconds since 1970, its size in bytes, whether it's a video or audio,
// and each option mpv kept for it, a tab between each. Each is { path,
// start, length, at, size, kind, kept }, `at` in milliseconds, as the
// shell keeps a moment, and `kept` its options; a line that doesn't read
// so is passed over.
function stopped(lines) {
    const read = [];
    for (const line of lines) {
        const [path, start, length, at, size, kind, ...rest] = line.split("\t");
        const one = {
            path,
            start: Number(start),
            length: Number(length),
            at: Number(at) * MILLISECONDS,
            size: Number(size),
            kind,
            kept: options(rest)
        };
        const figures = [start, length, at, size].every(part => (part ?? "") !== "")
            && [one.start, one.length, one.at, one.size].every(Number.isFinite);
        if (path.startsWith("/") && figures && (kind === "video" || kind === "audio")) {
            read.push(one);
        }
    }
    return read;
}

// Whether mpv was left on a file at its end, so it was watched whole and
// plays again from its start, rather than part way, where it plays on.
function whole(one) {
    return one.start > one.length - END;
}

// How much of a file is still to play from where it stopped, in seconds.
function toGo(one) {
    return Math.max(0, one.length - one.start);
}

// What mpv's drawer in Programs says of what it left part way, by its id
// in the launcher's words (VKCF3Z, 2026-11-08): how many videos, then how
// many audio files, "2 videos \u00b7 1 audio file", in `said`'s words; one
// watched whole isn't left to finish, and nothing is said while none is.
function held(left, said) {
    const partway = left.filter(one => !whole(one));
    const videos = partway.filter(one => one.kind === "video").length;
    const audio = partway.length - videos;
    const counted = (count, one, many) => {
        if (count === 0) {
            return "";
        }
        return String((count === 1 ? one : many) ?? "").replace("{count}", String(count));
    };
    const value = [counted(videos, said.video1, said.videos),
        counted(audio, said.audio1, said.audios)].filter(part => part !== "").join(" \u00b7 ");
    return value === "" ? [] : [{ id: "mpv:to-finish", value }];
}

// The words mpv's rows and drawers say, each thing's as `of` gives it, as
// Words.of does, and a day's as `date(date, format)` says it, as
// Qt.formatDate names days and months in the locale's words: what Resume's
// entry says (`said`), the labels of a plate's facts, and the words a
// moment is said in, as FileFacts.when takes them.
function wordsOf(of, date) {
    const said = of("verb:mpv:on").said ?? {};
    const name = id => of(id).name ?? "";
    return {
        said,
        toGo: name("mpv:stopped:to-go"),
        labels: {
            speed: name("mpv:stopped:speed"),
            volume: name("mpv:stopped:volume"),
            aid: name("mpv:stopped:audio"),
            sid: name("mpv:stopped:subtitles")
        },
        size: name("mpv:stopped:size"),
        path: name("mpv:stopped:path"),
        when: {
            ago: { now: said.now, minutes: said.minutes },
            days: { yesterday: said.yesterday, date }
        }
    };
}

// What a file's row says beside its name: where it stopped, its length
// and when mpv left it, "12:04 of 41:10 · just now", or for one
// watched whole its length and when, "41:10 · Friday", by the
// launcher's words for each (words.json's "stopped" and "whole"); `now`
// in milliseconds. Its folder is the drawer's eyebrow's instead.
function side(one, now, words) {
    const said = whole(one) ? words.said.whole : words.said.stopped;
    return String(said ?? "")
        .replace("{at}", clock(one.start))
        .replace("{length}", clock(one.length))
        .replace("{when}", FileFacts.when(one.at, now, words.when));
}

// What mpv kept of a file, the drawer's FIRST ones before the rest, each
// group in the order mpv wrote it.
function kept(one) {
    const first = (one.kept ?? []).filter(each => FIRST.includes(each.option))
        .sort((mine, theirs) => FIRST.indexOf(mine.option) - FIRST.indexOf(theirs.option));
    return first.concat((one.kept ?? []).filter(each => !FIRST.includes(each.option)));
}

// A kept option's value as the drawer says it: a figure without mpv's
// trailing zeros, a speed as so many times ("1.5×") and a volume as
// a share ("80%"), by the words' "speed" and "volume", a delay in
// seconds to the millisecond ("0.500 s") by "seconds", a track turned
// off as "off", and anything else as mpv wrote it, less any length mpv
// quoted it by.
function keptValue(each, said) {
    const plain = String(each.value ?? "").replace(QUOTED, "");
    const figure = FIGURE.test(plain) ? String(Number(Number(plain).toFixed(DECIMALS))) : null;
    if (figure !== null && (each.option === "speed" || each.option === "volume")) {
        return String(said[each.option] ?? "{figure}").replace("{figure}", figure);
    }
    if (figure !== null && IN_SECONDS.includes(each.option)) {
        return String(said.seconds ?? "{figure}")
            .replace("{figure}", Number(plain).toFixed(DECIMALS));
    }
    if ((each.option === "aid" || each.option === "sid") && plain === "no") {
        return said.off ?? plain;
    }
    return figure ?? plain;
}

// A file's drawer, `shown` its name, its folder and its path as the
// launcher shows them (Files.qml): its eyebrow where it is and then its
// kind, "in ~/Videos · video", as written, since a folder is the
// world's name, never its state or when, which its row's side says; its
// name; then how much is still to play, for one stopped part way, since
// that moves most; what mpv kept, each under mpv's own name where the
// words give it none; its size; and its path last, an identifier, in the
// Name face (VKCF3Z, 2026-11-08). Where it stopped and its length are
// its side's, so they aren't said again here.
function plate(one, shown, words) {
    const said = words.said;
    const left = whole(one) ? [] : [{ label: words.toGo, value: clock(toGo(one)) }];
    const asKept = kept(one).map(each => ({
        label: words.labels[each.option] ?? each.option,
        value: keptValue(each, said)
    }));
    return {
        eyebrow: String(said.eyebrow ?? "").replace("{folder}", shown.folder)
            .replace("{kind}", said[one.kind] ?? one.kind),
        eyebrowNames: true,
        title: shown.name,
        facts: left.concat(asKept, [
            { label: words.size, value: FileFacts.weighed(one.size) },
            // An identifier, drawn in the Name face once the drawer sets one
            // there (`identifies`, as Drive's id), in Figure until then.
            { label: words.path, value: shown.path, identifies: true }
        ])
    };
}

// What plays a file or an address in a window of its own, as mpv's desktop
// entry does: playing on from where it stopped, which mpv does by itself,
// or from its start, which leaves where it stopped unread.
function play(target, fromStart) {
    return ["mpv", "--player-operation-mode=pseudo-gui"]
        .concat(fromStart ? ["--no-resume-playback"] : [], ["--", target]);
}
