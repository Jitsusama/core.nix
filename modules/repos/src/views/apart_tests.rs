//! How far each head is from the branch it follows, as the views' own
//! record (views/tests.rs) says it at every width, and what a counting
//! says it counted.

use super::*;
use crate::views::tests::{NOW, ROOMS, clock, home, record};
use repos::model::Count;

/// A head following origin's main, at `count` from it where counted, or
/// with its upstream gone.
fn following(head: &mut Head, commit: Option<&str>, count: Option<Count>) {
    head.upstream = Some(Upstream {
        branch: "origin/main".to_owned(),
        commit: commit.map(str::to_owned),
        count,
    });
}

#[test]
fn each_head_says_how_far_it_is_from_the_branch_it_follows() {
    let mut record = record();
    let core = &mut record.repositories[0];
    let far = |behind, ahead| Some(Count { behind, ahead });
    let other = "89abcdef0123456789abcdef0123456789abcdef";
    following(
        core.checkouts[0].head.as_mut().expect("its head"),
        Some(other),
        far(3, 1),
    );
    let cut = &mut core.checkouts[0].trees;
    following(&mut cut[0].head, Some(other), far(COUNTED_TO, 0));
    following(&mut cut[1].head, None, None);
    following(&mut cut[2].head, Some(other), Some(Count::EVEN));
    following(&mut cut[3].head, Some(other), None);
    let core = &record.repositories[0];
    for room in ROOMS {
        let text = repository(&record, core, &clock(), Some(home()), &Pen::plain(), room);
        assert_eq!(layout::overrun(&text, room), None);
        let text = trees(core, &clock(), Some(home()), &Pen::plain(), room);
        assert_eq!(layout::overrun(&text, room), None);
    }
    let text = repository(
        &record,
        core,
        &clock(),
        Some(home()),
        &Pen::plain(),
        Room::Unbounded,
    );
    for said in [
        "main · 3 behind · 1 ahead",
        "jitsusama/change-0 · 100+ behind ",
        "jitsusama/change-1 · upstream gone ",
    ] {
        assert!(text.contains(said), "{said} isn't said: {text}");
    }
    assert!(
        !text.contains("change-2 ·") && !text.contains("change-3 ·"),
        "an even or uncounted head said a distance: {text}"
    );
}

#[test]
fn a_counting_says_how_many_heads_it_found_apart_and_behind() {
    let counted = Counted {
        schema: repos::upstreams::SCHEMA.to_owned(),
        counted_at: NOW,
        counts: [
            (
                "a...b",
                Count {
                    behind: 2,
                    ahead: 0,
                },
            ),
            (
                "a...c",
                Count {
                    behind: 0,
                    ahead: 1,
                },
            ),
        ]
        .into_iter()
        .map(|(key, count)| (key.to_owned(), count))
        .collect(),
    };
    assert_eq!(
        counting(&counted),
        "2 heads counted apart from their branches · 1 behind\n"
    );
    let answer = counting_json(&counted);
    assert_eq!(answer["schema"], "repos.upstreams/1");
    assert_eq!(answer["counted_at"], "2033-05-18T03:33:20Z");
    assert_eq!(answer["counts"]["a...b"]["behind"], 2);
}
