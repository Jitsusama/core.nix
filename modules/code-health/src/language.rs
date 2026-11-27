//! The language a file is written in, by its extension: the one table
//! every measure asks (measure.rs, reached.rs and stale.rs), so each reads
//! a language alike, in every extension its files take, and one added
//! here is read by all of them.

use crate::measure::extension;

/// A language whose files are counted as code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    /// C and C++, with their headers.
    C,
    Go,
    Lua,
    Nix,
    /// Python, with its type stubs.
    Python,
    Qml,
    Ruby,
    Rust,
    /// JavaScript and TypeScript, in each module form and with JSX, since
    /// their comments, suppressions and imports are written alike.
    Script,
    /// sh and bash.
    Shell,
    Terraform,
}

/// The language a path's file is written in, or `None` for a file that
/// isn't code.
pub fn language(path: &str) -> Option<Language> {
    Some(match extension(path) {
        "c" | "cc" | "cpp" | "h" => Language::C,
        "go" => Language::Go,
        "lua" => Language::Lua,
        "nix" => Language::Nix,
        "py" | "pyi" => Language::Python,
        "qml" => Language::Qml,
        "rb" => Language::Ruby,
        "rs" => Language::Rust,
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "mts" | "cts" | "tsx" => Language::Script,
        "sh" | "bash" => Language::Shell,
        "tf" => Language::Terraform,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use crate::measure::{File, LONG, file, measure};
    use crate::reached::unreached;
    use crate::stale::stale;

    fn paths(files: &[File]) -> Vec<String> {
        files.iter().map(|file| file.path.clone()).collect()
    }

    #[test]
    fn a_language_is_counted_in_every_extension_its_files_take() {
        let files = [
            file("a.pyi", "def f() -> int: ...  # type: ignore\n"),
            file("b.mts", "// @ts-expect-error: why\n"),
            file("c.cts", "// eslint-disable-next-line\n"),
            file("d.jsx", "// biome-ignore lint: why\n"),
            file("e.cc", "// biome-ignore lint: not C's\n"),
            file("f.mts", &"let f = 1;\n".repeat(LONG + 1)),
        ];
        let health = measure(&paths(&files), &files, &[]);
        assert_eq!(
            health.named["suppressions"],
            ["a.pyi:1", "b.mts:1", "c.cts:1", "d.jsx:1"],
            "a suppression in one of its language's extensions was missed, or another's counted"
        );
        assert_eq!(
            health.named["long-files"],
            ["f.mts"],
            "a long file in one of its language's extensions was missed"
        );
    }

    #[test]
    fn a_file_is_reached_as_its_language_reaches_it_in_every_extension() {
        let files = [
            file(
                "web/app.ts",
                "import { c } from \"./c.mjs\";\nimport d from './d';\n",
            ),
            file("web/c.mts", "\n"),
            file("web/d.cts", "\n"),
            file("web/e.jsx", "\n"),
            file("py/run.py", "from tally import count\n"),
            file("py/tally.pyi", "\n"),
            file("py/plain.pyi", "\n"),
        ];
        assert_eq!(
            unreached(&paths(&files), &files),
            ["web/app.ts", "web/e.jsx", "py/run.py", "py/plain.pyi"],
            "a file of one of its language's extensions went uncounted, or its import unread"
        );
    }

    #[test]
    fn a_comment_is_read_in_every_extension_its_language_takes() {
        let files = [
            file("tools/a.mjs", "// Read by tools/kept/a.sh\n"),
            file("tools/b.cjs", "// Read by tools/kept/b.sh\n"),
            file("tools/c.jsx", "// Read by tools/kept/c.sh\n"),
            file("tools/d.mts", "/* Read by\n * tools/kept/d.sh */\n"),
            file("tools/e.cts", "// Read by tools/kept/e.sh\n"),
            file("tools/f.tsx", "// Read by tools/kept/f.sh\n"),
            file("tools/g.cc", "// Read by tools/kept/g.sh\n"),
            file("tools/h.cpp", "// Read by tools/kept/h.sh\n"),
            file("tools/i.pyi", "# Read by tools/kept/i.sh\n"),
            file("tools/kept/x.nix", "{ }\n"),
        ];
        assert_eq!(
            stale(&paths(&files), &files),
            [
                "tools/a.mjs:1: tools/kept/a.sh",
                "tools/b.cjs:1: tools/kept/b.sh",
                "tools/c.jsx:1: tools/kept/c.sh",
                "tools/d.mts:2: tools/kept/d.sh",
                "tools/e.cts:1: tools/kept/e.sh",
                "tools/f.tsx:1: tools/kept/f.sh",
                "tools/g.cc:1: tools/kept/g.sh",
                "tools/h.cpp:1: tools/kept/h.sh",
                "tools/i.pyi:1: tools/kept/i.sh"
            ],
            "a comment in one of its language's extensions went unread"
        );
    }
}
