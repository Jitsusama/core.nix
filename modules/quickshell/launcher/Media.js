// What a media player's drawer shows, from everything it says over MPRIS,
// in plain JavaScript so tests/media.qml holds it in Qt's own engine over
// players it makes up, shaped as Quickshell's MprisPlayer is. Every row that
// stands for a player draws it: Now's Playing row (pieces/Playing.qml), a
// program's Media part (Applications.qml) and the players' own place.
//
// It's set as every plate is (VKCF3Z, 2026-11-07): the eyebrow where it
// plays and what it is ("on Spotify · track"), never how it stands, which
// the row's side says; the track as the title; its art under that; then
// the facts, what moves most first (where in the track, the volume, shuffle,
// loop and speed), then what describes it (artist, album, album artist,
// track and disc, genre, rating), and what identifies it last (its link
// and the player's bus name); the track's words, its lyrics, when it gives
// them. A fact the player doesn't support or doesn't say is left out, never
// shown as unknown, and what it can do isn't listed, since its acts say it.
.pragma library
.import "../Music.js" as Players

// MprisLoopState's values, as Quickshell numbers them.
const LOOPS = ["off", "track", "playlist"];
// A rate said to two places at most, as players offer them: 1.25×.
const PLACES = 100;

// A remote picture, which the network gives, and only over TLS: lookups
// say the network may be asked, not that it may be asked in the clear
// (Y2YBTS, 2026-11-08), so art at an http address is never drawn.
const REMOTE = /^https:/i;
// One on the machine, or held in the address itself, as mpv's gives the
// art a file carries: neither asks the network.
const LOCAL = /^(?:file:\/\/|data:image\/)/i;

// "{n}" in a sentence the vocabulary gives, filled.
function filled(words, fill) {
    return Object.keys(fill).reduce((said, key) => said.split(`{${key}}`).join(fill[key]),
        String(words ?? ""));
}

// A rate as a player says one: 1×, 0.5×, 1.25×.
function rate(value) {
    return `${Math.round(value * PLACES) / PLACES}\u00d7`;
}

// A share of one as a whole percentage: 0.64 is 64%.
function percent(value) {
    return `${Math.round(value * 100)}%`;
}

// A metadata value MPRIS gives as a list or as one: the genres, the album
// artists. Joined as a list is said, or "" for none.
function listed(value) {
    if (value === undefined || value === null) {
        return "";
    }
    const each = Array.isArray(value) ? value : typeof value === "object"
        ? Array.from(value) : [value];
    return each.map(one => String(one).trim()).filter(one => one !== "").join(", ");
}

// Where in the track it is: "1:38 of 6:43", or how far it's come for a
// stream that has no length. Nothing for a player that can't say.
function position(player) {
    if (player.positionSupported === false) {
        return "";
    }
    const length = player.lengthSupported === false ? 0 : Number(player.length) || 0;
    const at = Players.time(Number(player.position) || 0);
    return length > 0 ? `${at} of ${Players.time(length)}` : at;
}

// Its track number and disc, "3 · disc 1", from what the metadata says.
function track(metadata, said) {
    const number = Number(metadata["xesam:trackNumber"]) || 0;
    const disc = Number(metadata["xesam:discNumber"]) || 0;
    const discSaid = disc > 0 ? filled(said.disc, { disc }) : "";
    return [number > 0 ? String(number) : "", discSaid].filter(one => one !== "")
        .join(" \u00b7 ");
}

// Its rate, only where it can change: "1× · 0.5× to 2×".
function rates(player, said) {
    const least = Number(player.minRate);
    const most = Number(player.maxRate);
    if (!(most > least)) {
        return "";
    }
    return `${rate(Number(player.rate) || 1)} \u00b7 `
        + filled(said.range, { least: rate(least), most: rate(most) });
}

// The picture the drawer shows of the track: its art, as the player gives
// it, but a remote one only where the launcher may ask the network, and
// nothing for a scheme no picture is drawn from.
function art(url, lookups) {
    const given = String(url ?? "");
    if (LOCAL.test(given)) {
        return given;
    }
    if (REMOTE.test(given)) {
        return lookups ? given : "";
    }
    return "";
}

// Its facts, in order, each { id, value } with a value only when the
// player says one; `said` holds the vocabulary's words for the values
// (words.json's media's `said`).
function facts(player, said) {
    const metadata = player.metadata ?? {};
    const artist = String(player.trackArtist ?? "").trim();
    const albumArtist = listed(metadata["xesam:albumArtist"])
        || String(player.trackAlbumArtist ?? "").trim();
    const rating = Number(metadata["xesam:userRating"]);
    const shuffle = player.shuffleSupported === true ? (player.shuffle ? said.on : said.off)
        : "";
    const loop = player.loopSupported === true ? said[LOOPS[player.loopState] ?? ""] ?? "" : "";
    const parts = [
        { id: "position", value: position(player) },
        { id: "volume", value: player.volumeSupported === true
            ? percent(Number(player.volume) || 0) : "" },
        { id: "shuffle", value: shuffle },
        { id: "loop", value: loop },
        { id: "speed", value: rates(player, said) },
        { id: "fullscreen", value: player.canSetFullscreen === true
            ? player.fullscreen ? said.on : said.off : "" },
        { id: "artist", value: artist },
        { id: "album", value: String(player.trackAlbum ?? "").trim() },
        { id: "album-artist", value: albumArtist === artist ? "" : albumArtist },
        { id: "track", value: track(metadata, said) },
        { id: "genre", value: listed(metadata["xesam:genre"]) },
        { id: "rating", value: Number.isFinite(rating) ? percent(rating) : "" },
        { id: "link", value: String(metadata["xesam:url"] ?? "").trim() },
        { id: "bus-name", value: String(player.dbusName ?? "") }
    ];
    return parts.filter(part => part.value !== "");
}

// The player's drawer, given the names of its facts by id (`named`), the
// vocabulary's words for its values (`said`) and whether the launcher may
// ask the network, for remote art (`lookups`).
function drawer(player, named, said, lookups) {
    const metadata = player.metadata ?? {};
    const picture = art(player.trackArtUrl, lookups);
    const title = String(player.trackTitle ?? "").trim();
    return {
        eyebrow: filled(said.eyebrow, { player: player.identity ?? "" }),
        // It names the player by its own name, so it's set as written, never
        // in the label's capitals (VKCF3Z, 2026-11-08).
        eyebrowNames: true,
        title: title === "" ? String(player.identity ?? "") : title,
        preview: picture === "" ? null : { picture, across: 0, down: 0 },
        facts: facts(player, said).map(part => ({ label: named(part.id), value: part.value })),
        body: String(metadata["xesam:asText"] ?? "").trim()
    };
}
