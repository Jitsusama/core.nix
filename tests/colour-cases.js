// The colours tests/answers.qml asks the launcher about, kept apart from it
// because they name colours, which nothing outside the theme may but the
// launcher's Colours.js and this.
.pragma library

// A few of a theme's pigments and the roles that wear them.
const palette = { vermilion: "#FF5345", jade: "#509475", night: "#090f0d", moss: "#32473B" };
const roles = {
    alert: "#FF5345",
    accent: "#509475",
    deepest: "#090f0d",
    shadow: "#090f0d",
    edge: "#32473B",
    selection: "#32473B"
};

// What's typed, the answer in the field, and its rows, each with its side.
const answers = [
    ["#ff0000", "rgb(255 0 0)", ["hsl(0 100% 50%)", "oklch(62.8% 0.258 29.2)",
        "vermilion alert \u00b7 \u0394E 6.9"]],
    ["#ff5345", "rgb(255 83 69)", ["hsl(4.5 100% 63.5%)", "oklch(67.8% 0.21 28.6)",
        "vermilion alert"]],
    ["#F53", "rgb(255 85 51)", ["hsl(10 100% 60%)", "oklch(67.8% 0.212 33.2)",
        "vermilion alert \u00b7 \u0394E 1.7"]],
    ["rgba(255, 83, 69, 0.5)", "#ff534580", ["hsl(4.5 100% 63.5% / 0.5)",
        "oklch(67.8% 0.21 28.6 / 0.5)", "vermilion alert"]],
    ["hsl(120deg 100% 50%)", "#00ff00", ["rgb(0 255 0)", "oklch(86.6% 0.295 142.5)"]],
    ["oklch(62.8% 0.2577 29.23)", "#ff0000", ["rgb(255 0 0)", "hsl(0 100% 50%)",
        "vermilion alert \u00b7 \u0394E 6.9"]],
    ["#808080", "rgb(128 128 128)", ["hsl(0 0% 50.2%)", "oklch(60% 0 none)",
        "jade accent \u00b7 \u0394E 8.6"]],
    ["colour accent", "#509475", ["rgb(80 148 117)", "hsl(152.6 29.8% 44.7%)",
        "oklch(61.2% 0.085 162.8)", "jade accent"]],
    ["color edge", "#32473b", ["rgb(50 71 59)", "hsl(145.7 17.4% 23.7%)",
        "oklch(37.6% 0.033 159.2)", "moss edge \u00b7 selection"]]
];

// What's typed, and the swatch its first row wears, alpha first as Qt
// reads it.
const swatch = ["#ff534580", "#80ff5345"];

// What isn't a colour, so nothing answers.
const not = ["rgb(300 0 0)", "#ggg", "rgb", "colour narnia", "#12345", "red", "hsl(1 2)"];
