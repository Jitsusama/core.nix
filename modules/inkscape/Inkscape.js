// What Inkscape's provider in the launcher works out (InkscapeProgram.qml),
// kept apart from Quickshell's types so tests/inkscape-program.qml runs it
// in Qt's own engine: the drawings Inkscape opened lately, as
// inkscape-drawings (drawings.sh) says each still there, which the
// programs daemon runs and holds for it; each
// drawing's size as its SVG says it, its row's side and its drawer's
// plate; what Inkscape's drawer in Programs says of them; what the
// clipboard holds that a new drawing can be made from; and the commands
// that make one. A file's weight and when it was opened are said as the
// launcher says them of any file (FileFacts.js).
.pragma library
.import "../FileFacts.js" as FileFacts

// An element's attributes, each a name and its value in double quotes,
// as an SVG's root is written, and the entities a value is escaped with.
const ATTRIBUTE = /[\w:-]+="[^"]*"/g;
const ENTITY = /&(amp|lt|gt|quot|apos|#x[\da-f]+|#\d+);/gi;
const ENTITIES = { amp: "&", lt: "<", gt: ">", quot: "\"", apos: "'" };
const HEX = 16;
const DECIMAL = 10;
// Where a value starts after its name, past the equals sign and the quote.
const OPENING = 2;

// The microseconds in a millisecond, as inkscape-drawings says when to
// the microsecond and the launcher counts in milliseconds.
const MICROSECONDS = 1000;

// A length as an SVG's width or height says it, a figure and its unit,
// none being the user's unit, a pixel; a share of something else, as
// 100%, isn't one.
const LENGTH = /^\s*([+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?)\s*(px|mm|cm|in|pt|pc|q)?\s*$/i;
// A viewBox's four figures, by spaces or commas, its width and height last.
const BOX_GAPS = /[\s,]+/;
const BOX = 4;
const BOX_WIDTH = 2;
const BOX_HEIGHT = 3;
// The most decimals a size is said with.
const DECIMALS = 3;
// A figure keeps to its unit with a no-break space, as FileFacts.js does.
const KEPT = "\u00a0";
const BETWEEN = " \u00b7 ";

// The types Inkscape takes a drawing from the clipboard in, its own first,
// which keeps its layers, then any program's SVG.
const VECTORS = ["image/x-inkscape-svg", "image/svg+xml"];

// What a new drawing's file is named, then when it was made, as GIMP names
// a copied picture's, so its window's title says which it is.
const FRESH_NAME = "Drawing";
const COPIED_NAME = "Copied";

// A value as it was before it was escaped.
function unescaped(value) {
    return value.replace(ENTITY, (whole, name) => {
        if (!name.startsWith("#")) {
            return ENTITIES[name.toLowerCase()] ?? whole;
        }
        const hex = name[1] === "x" || name[1] === "X";
        return String.fromCodePoint(Number.parseInt(name.slice(hex ? OPENING : 1),
            hex ? HEX : DECIMAL));
    });
}

// An element's attributes by name, from its start tag.
function attributes(tag) {
    const read = {};
    for (const pair of String(tag ?? "").match(ATTRIBUTE) ?? []) {
        const at = pair.indexOf("=");
        read[pair.slice(0, at)] = unescaped(pair.slice(at + OPENING, -1));
    }
    return read;
}

// A length as an SVG's root says it, { figure, unit }, or null for none, a
// share or a size of nothing.
function length(said) {
    const parts = LENGTH.exec(String(said ?? ""));
    if (!parts || Number(parts[1]) <= 0) {
        return null;
    }
    const unit = (parts[2] ?? "px").toLowerCase();
    return { figure: Number(parts[1]), unit: unit === "q" ? "Q" : unit };
}

// A drawing's size as its root element says it, { width, height }, each a
// length: its width and height where both are lengths, else its viewBox's,
// in the user's unit, a pixel, as Inkscape takes it, or null where it says
// neither.
function extent(tag) {
    const root = attributes(tag);
    const width = length(root.width);
    const height = length(root.height);
    if (width !== null && height !== null) {
        return { width, height };
    }
    const box = String(root.viewBox ?? "").trim().split(BOX_GAPS).map(Number);
    if (box.length !== BOX || !box.every(Number.isFinite)
        || box[BOX_WIDTH] <= 0 || box[BOX_HEIGHT] <= 0) {
        return null;
    }
    return {
        width: { figure: box[BOX_WIDTH], unit: "px" },
        height: { figure: box[BOX_HEIGHT], unit: "px" }
    };
}

function figure(number) {
    return String(Number(number.toFixed(DECIMALS)));
}

// A drawing's size as its drawer says it, its unit once where both share
// one, "210 \u00d7 297 mm", or each with its own, "8.5 in \u00d7 280 mm",
// or "" where its SVG says none.
function dimensions(size) {
    if (size === null || size === undefined) {
        return "";
    }
    const { width, height } = size;
    if (width.unit === height.unit) {
        return `${figure(width.figure)} \u00d7 ${figure(height.figure)}${KEPT}${width.unit}`;
    }
    return `${figure(width.figure)}${KEPT}${width.unit} \u00d7 `
        + `${figure(height.figure)}${KEPT}${height.unit}`;
}

// Whether a field inkscape-drawings printed is a number.
function figured(field) {
    return (field ?? "") !== "" && Number.isFinite(Number(field));
}

// The drawings still there, as inkscape-drawings prints them, a line each:
// its whole path, when Inkscape last opened it in microseconds since 1970,
// its weight in bytes and its root element's start tag, a tab between
// each; each { path, at, size, extent }, `at` in milliseconds to the
// microsecond, newest first, each once, at most `most`. A line without a
// time or a weight is passed over.
function drawings(lines, most) {
    const read = new Map();
    for (const line of lines ?? []) {
        const [path, at, weight, ...tag] = String(line).split("\t");
        if (path !== "" && figured(at) && figured(weight) && !read.has(path)) {
            read.set(path, { path, at: Number(at) / MICROSECONDS, size: Number(weight),
                extent: extent(tag.join(" ")) });
        }
    }
    return Array.from(read.values()).sort((one, other) => other.at - one.at).slice(0, most);
}

// The words its rows and drawers say, each thing's as `of` gives it, as
// Words.of does, and a day's as `date(date, format)` says it, as
// Qt.formatDate names days and months in the locale's words: what New
// Drawing's entry says (`said`), the labels of a plate's facts, and the
// words a moment is said in, as FileFacts.when takes them.
function wordsOf(of, date) {
    const said = of("verb:inkscape:drawing").said ?? {};
    const name = id => of(id).name ?? "";
    return {
        said,
        dimensions: name("inkscape:dimensions"),
        size: name("inkscape:size"),
        path: name("inkscape:path"),
        when: {
            ago: { now: said.now, minutes: said.minutes },
            days: { yesterday: said.yesterday, date }
        }
    };
}

// What a drawing's row says beside its name: when Inkscape last opened it,
// `now` in milliseconds. Its folder is its drawer's eyebrow's instead.
function side(one, now, words) {
    return FileFacts.when(one.at, now, words.when);
}

// A drawing's drawer, `shown` its name, folder and path as the launcher
// shows them (Files.qml): its eyebrow where it is and then its kind, "in
// ~/Pictures/Drawings \u00b7 drawing", as written, since a folder is a
// name; its name; then its size as its SVG says it, where it says one,
// its weight, and its path last, an identifier. When it was opened is its
// row's side, so it isn't said again here.
function plate(one, shown, words) {
    const { said } = words;
    const size = dimensions(one.extent);
    const sized = size === "" ? [] : [{ label: words.dimensions, value: size }];
    return {
        eyebrow: String(said.eyebrow ?? "").replace("{folder}", shown.folder)
            .replace("{kind}", said.drawing ?? ""),
        eyebrowNames: true,
        title: shown.name,
        facts: sized.concat([
            { label: words.size, value: FileFacts.weighed(one.size) },
            { label: words.path, value: shown.path, identifies: true }
        ])
    };
}

// What Inkscape's drawer in Programs says of its drawings lately, by each
// fact's id in the launcher's words, as Tuple's says of its calls: the
// last one Inkscape opened, its name and when, "logo.svg \u00b7 12 min
// ago", then how many it opened lately. Nothing while it opened none.
function held(lately, now, words) {
    if (lately.length === 0) {
        return [];
    }
    const [last] = lately;
    const name = last.path.slice(last.path.lastIndexOf("/") + 1);
    return [
        { id: "inkscape:last", value: `${name}${BETWEEN}${side(last, now, words)}` },
        { id: "inkscape:lately", value: String(lately.length) }
    ];
}

// What the clipboard holds that a new drawing is made from, or null: an
// SVG, by the type it's offered in, wherever it's offered as one, as
// Inkscape and a browser offer what they copy; else the picture the
// shell's watch of it says (Copied.qml's `picture`) while the clipboard's
// history keeps its width and height (`newest`, launcher/Copies.qml's
// copy), which the drawing is made the size of, so a picture the history
// doesn't hold makes none. { type, vector, across, down, about }.
function copied(types, picture, newest) {
    const vector = VECTORS.find(type => (types ?? []).includes(type));
    if (vector !== undefined) {
        return { type: vector, vector: true, across: 0, down: 0, about: "" };
    }
    const across = Number(newest?.across ?? 0);
    const down = Number(newest?.down ?? 0);
    if ((picture ?? "") === "" || !newest?.picture || !(across > 0 && down > 0)) {
        return null;
    }
    return { type: picture, vector: false, across, down, about: newest.about ?? "" };
}

// What a copy is beside New Drawing From the Copied Picture: SVG, by the
// words' `svg`, or a picture's width and height and what the history
// says of it, as GIMP's act says it, "256 \u00d7 256 \u00b7 PNG \u00b7 1.2 kB".
function copiedSaid(copy, said) {
    if (copy.vector) {
        return said.svg ?? "";
    }
    return `${copy.across} \u00d7 ${copy.down}${BETWEEN}${copy.about}`;
}

// What a new drawing's file is named, but for its .svg: in `folder`,
// named for what it's from and `stamp`, when it was made.
function fileFor(folder, ofCopy, stamp) {
    return `${folder}/${ofCopy ? COPIED_NAME : FRESH_NAME} ${stamp}`;
}

// The start of each command making a new drawing, through `sh` as niri
// spawns it, given its folder ($1) and its file's name but for its .svg
// ($3): the folder made, and the file named so, or with a count after it
// where one already is, as two made in one second are, never written
// over.
const NAMED = "mkdir -p -- \"$1\" && n=1 && f=\"$3.svg\" && while [ -e \"$f\" ];"
    + " do n=$((n + 1)); f=\"$3 $n.svg\"; done && set -C && ";

// What opens a new drawing, a copy of the template written first so
// Inkscape opens it whole and never the template itself: `file` its name
// but for its .svg, and `tools` { folder, template, inkscape }.
function fresh(file, tools) {
    return ["sh", "-c", `${NAMED}cat -- "$2" > "$f" && exec "$4" "$f"`, "sh", tools.folder,
        tools.template, file, tools.inkscape];
}

// The SVG a picture is made a drawing in, at the picture's own size in
// pixels, the page and its desk in the theme's colours, as the template
// has them, and the picture in a layer of its own, each {name} filled in;
// the picture follows its start, in base64, and then its end.
const PICTURE_START = "<svg xmlns=\"http://www.w3.org/2000/svg\""
    + " xmlns:sodipodi=\"http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd\""
    + " xmlns:inkscape=\"http://www.inkscape.org/namespaces/inkscape\""
    + " width=\"{across}\" height=\"{down}\" viewBox=\"0 0 {across} {down}\">\n"
    + "  <sodipodi:namedview bordercolor=\"{border}\" inkscape:deskcolor=\"{desk}\" />\n"
    + "  <g inkscape:label=\"Layer 1\" inkscape:groupmode=\"layer\" id=\"layer1\">\n"
    + "    <image width=\"{across}\" height=\"{down}\" href=\"data:{type};base64,";
const PICTURE_END = "\" />\n  </g>\n</svg>\n";
const FILLED = /\{(across|down|border|desk|type)\}/g;

// What opens a new drawing made from the copy (`copied`), written first
// as Inkscape opens it whole: an SVG as it was copied, or a picture inside
// one, `file` its name but for its .svg, and `tools` { folder, paste,
// inkscape, border, desk }, `paste` being wl-paste.
function fromCopy(copy, file, tools) {
    const given = ["sh", tools.folder, tools.paste, file, tools.inkscape, copy.type];
    if (copy.vector) {
        return ["sh", "-c", `${NAMED}"$2" --type "$5" > "$f" && exec "$4" "$f"`].concat(given);
    }
    const filled = { across: copy.across, down: copy.down, border: tools.border,
        desk: tools.desk, type: copy.type };
    const head = PICTURE_START.replace(FILLED, (_whole, name) => String(filled[name]));
    return ["sh", "-c", `${NAMED}{ printf %s "$6" && "$2" --type "$5" | base64 -w 0`
        + ` && printf %s "$7"; } > "$f" && exec "$4" "$f"`].concat(given, [head, PICTURE_END]);
}
