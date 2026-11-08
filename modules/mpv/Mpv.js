// What mpv's provider in the launcher works out (MpvProgram.qml), kept apart
// from Quickshell's types so tests/mpv-program.qml runs it in Qt's own
// engine: whether what was copied is one address mpv plays, how far into a
// file mpv stopped, said in words, the files mpv-stopped lists (stopped.sh),
// and the commands that play each.
.pragma library

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

// What leads a host but doesn't name its site: "m.youtube.com" is
// youtube.com.
const LEADING = /^(?:www|m)\./;

const MINUTE = 60;
const HOUR = 3600;
const MILLISECONDS = 1000;

// An address's host, as its site is known by.
function siteOf(host) {
    return host.toLowerCase().replace(LEADING, "");
}

// The address mpv plays in what was copied, or "" when it holds anything
// else: more than one address, words around one, or an address of a page
// with nothing to play.
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
    const whole = Math.max(0, Math.floor(Number(seconds) || 0));
    const hours = Math.floor(whole / HOUR);
    const minutes = Math.floor((whole % HOUR) / MINUTE);
    const rest = String(whole % MINUTE).padStart(2, "0");
    const said = hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}` : String(minutes);
    return `${said}:${rest}`;
}

// The files mpv-stopped printed, a line each: the file's whole path, where
// it stopped and how long it is, both in seconds, and when mpv left it, in
// seconds since 1970, a tab between each. Each is { path, start, length,
// at }, `at` in milliseconds, as the shell keeps a moment; a line that
// doesn't read so is passed over.
function stopped(lines) {
    const read = [];
    for (const line of lines) {
        const [path, start, length, at] = line.split("\t");
        const one = {
            path,
            start: Number(start),
            length: Number(length),
            at: Number(at) * MILLISECONDS
        };
        const figures = [start, length, at].every(part => (part ?? "") !== "")
            && [one.start, one.length, one.at].every(Number.isFinite);
        if (path.startsWith("/") && figures) {
            read.push(one);
        }
    }
    return read;
}

// Where a file stopped, as its row says it, by the launcher's words for it
// (`said`, words.json's "{at} of {length}"): "12:04 of 41:10".
function where(one, said) {
    return String(said ?? "")
        .replace("{at}", clock(one.start))
        .replace("{length}", clock(one.length));
}

// What plays a file or an address in a window of its own, as mpv's desktop
// entry does: playing on from where it stopped, which mpv does by itself,
// or from its start, which leaves where it stopped unread.
function play(target, fromStart) {
    return ["mpv", "--player-operation-mode=pseudo-gui"]
        .concat(fromStart ? ["--no-resume-playback"] : [], ["--", target]);
}
