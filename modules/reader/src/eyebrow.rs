//! The eyebrow over a page's title: what the file says it is, or where it
//! lives, and when it last changed.

/// The eyebrow's words over the title: what the file says it is in its
/// front matter, its kind and id, or else where it lives; and when it last
/// changed. The title says the file's own name, and the front matter
/// itself never shows.
pub fn about(file: &std::path::Path, markdown: &str) -> String {
    let name = said(markdown).unwrap_or_else(|| lives(file));
    let changed = std::fs::metadata(file)
        .and_then(|it| it.modified())
        .ok()
        .and_then(|at| jiff::Timestamp::try_from(at).ok());
    let Some(changed) = changed else { return name };
    let zone = jiff::tz::TimeZone::system();
    let now = jiff::Timestamp::now().to_zoned(zone.clone());
    format!(
        "{name} \u{b7} changed {}",
        when(&changed.to_zoned(zone), &now)
    )
}

/// Where a file lives: in a repository, its name and the folders down to
/// the file, each after a middot; elsewhere, its folder's name.
fn lives(file: &std::path::Path) -> String {
    let file = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    let Some(folder) = file.parent() else {
        return String::new();
    };
    let named = |path: &std::path::Path| {
        path.file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
    };
    let Some(root) = folder.ancestors().find(|it| it.join(".git").exists()) else {
        return named(folder);
    };
    let below = folder
        .strip_prefix(root)
        .unwrap_or(std::path::Path::new(""));
    std::iter::once(named(root))
        .chain(below.iter().map(|part| part.to_string_lossy().into_owned()))
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

/// What a file's front matter says it is: its kind and its id, whichever
/// it gives, joined as the eyebrow joins its parts.
fn said(markdown: &str) -> Option<String> {
    let rest = markdown.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    let field = |key: &str| {
        rest[..end].lines().find_map(|line| {
            let value = line.strip_prefix(key)?.strip_prefix(':')?.trim();
            let value = value.trim_matches(|c| c == '"' || c == '\'');
            (!value.is_empty()).then(|| value.to_string())
        })
    };
    let parts: Vec<String> = [field("kind"), field("id")].into_iter().flatten().collect();
    (!parts.is_empty()).then(|| parts.join(" \u{b7} "))
}

/// When a file changed, as briefly as says it: a clock's time today, a day
/// this year, a day and its year before that.
fn when(changed: &jiff::Zoned, now: &jiff::Zoned) -> String {
    let form = if changed.date() == now.date() {
        "%H:%M"
    } else if changed.year() == now.year() {
        "%-d %b"
    } else {
        "%-d %b %Y"
    };
    changed.strftime(form).to_string()
}

#[cfg(test)]
mod tests {
    use super::{lives, said, when};

    fn at(year: i16, month: i8, day: i8, hour: i8, minute: i8) -> jiff::Zoned {
        jiff::civil::date(year, month, day)
            .at(hour, minute, 0, 0)
            .to_zoned(jiff::tz::TimeZone::fixed(jiff::tz::offset(-5)))
            .expect("a time in a fixed zone")
    }

    #[test]
    fn a_file_lives_in_its_repositorys_folders_or_else_its_own() {
        let dir = std::env::temp_dir().join(format!("reader-lives-{}", std::process::id()));
        let repo = dir.join("core.nix");
        std::fs::create_dir_all(repo.join(".git")).expect("a repository");
        std::fs::create_dir_all(repo.join("docs/notes")).expect("its folders");
        std::fs::create_dir_all(dir.join("loose")).expect("a folder");
        assert_eq!(lives(&repo.join("README.md")), "core.nix");
        assert_eq!(lives(&repo.join("docs/design.md")), "core.nix \u{b7} docs");
        assert_eq!(
            lives(&repo.join("docs/notes/a.md")),
            "core.nix \u{b7} docs \u{b7} notes"
        );
        assert_eq!(lives(&dir.join("loose/page.md")), "loose");
        std::fs::remove_dir_all(&dir).expect("cleaned");
    }

    #[test]
    fn a_file_says_what_it_is_in_its_front_matter() {
        assert_eq!(
            said("---\nid: QEST-1\nkind: subquest\nstatus: active\n---\n# Title\n"),
            Some("subquest \u{b7} QEST-1".into())
        );
        assert_eq!(
            said("---\ntitle: \"Tables\"\nid: 'tables'\n---\n"),
            Some("tables".into())
        );
        assert_eq!(said("---\ntitle: Tables\n---\n"), None);
        assert_eq!(said("# No front matter\n\nkind: words\n"), None);
    }

    #[test]
    fn a_change_is_said_as_briefly_as_says_when() {
        let now = at(2026, 11, 3, 9, 40);
        assert_eq!(when(&at(2026, 11, 3, 9, 14), &now), "09:14");
        assert_eq!(when(&at(2026, 11, 2, 23, 5), &now), "2 Nov");
        assert_eq!(when(&at(2025, 10, 30, 8, 0), &now), "30 Oct 2025");
    }
}
