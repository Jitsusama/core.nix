//! The object store's tests of what one find keeps: a base found by name
//! is found once however many copies name it, and a copy passed over is
//! passed over again only while what passed it over holds.

use super::delta_tests::{
    Entries, blob, delta_over, named, names, over, packed_as, read_each, within,
};
use super::tests::read_from;
use super::*;

#[test]
fn a_base_found_is_found_once_a_find_however_many_copies_name_it() {
    // Names 0 to 199 each a delta over the next, down to the blob 200, in
    // two packs: in the first, each says its base is of 2 bytes, so it
    // fails over the base it names once that's found; in the second, each
    // is sound. Each name's first copy fails and its second names the
    // same base again, which found again would take a lookup for each
    // name below it: about 200 squared lookups rather than 401.
    let rungs = 200;
    let names = names(rungs + 1);
    let failing = (0..rungs).map(|at| delta_over(names[at + 1], 2)).collect();
    let mut sound: Vec<Vec<u8>> = (0..rungs).map(|at| over(names[at + 1])).collect();
    sound.push(blob(b"a"));
    let folders = packed_as(&[(names[..rungs].to_vec(), failing), (names.clone(), sound)]);
    let objects = folders.0[0].clone();
    let read = within(20, move || {
        super::chain::LOOKUPS.with(|lookups| lookups.set(0));
        let read = read_from(&objects, names[0]);
        (read, super::chain::LOOKUPS.with(std::cell::Cell::get))
    });
    // Each name's two copies take a lookup each, and the blob's one more.
    assert_eq!(
        read,
        Ok((Ok(Some(b"x".to_vec())), 2 * rungs + 1)),
        "a base found for a copy that then failed was found again for the next copy naming it"
    );
}

#[test]
fn a_chain_too_long_from_deep_leaves_a_copy_naming_its_base_from_near_the_top_read() {
    // names[0]'s first copy names the top of a chain, names[limit], which
    // runs CHAIN_LIMIT - 1 deltas placed by offset down to a delta naming
    // the blob names[limit + 1]: the blob is then found CHAIN_LIMIT + 1
    // deltas down, past the limit. names[0]'s second copy names the blob
    // from the top, one delta down.
    let limit = CHAIN_LIMIT;
    let names = names(limit + 2);
    let (top, chained, blob_name) = (names[0], names[limit], names[limit + 1]);
    let delta = [1, 1, 1, b'x'];
    let mut entries = Entries::new();
    entries.push(over(chained));
    entries.push(named(blob_name, &delta));
    for at in 2..=limit {
        entries.place_over(at - 1, &delta);
    }
    // The first pack names its entries in the order they lie: names[0]
    // first, then the chain, its top at names[limit].
    let first = (names[..=limit].to_vec(), entries.entries);
    let second = (vec![top, blob_name], vec![over(blob_name), blob(b"a")]);
    let folders = packed_as(&[first, second]);
    let objects = folders.0[0].clone();
    let read = within(60, move || read_each(&objects, &[top]));
    assert_eq!(
        read,
        Ok((vec![Ok(Some(b"x".to_vec()))], 0)),
        "a base passed over for a chain too long from deep was passed over from near the \
         top, or a copy on the way was marked"
    );
}

#[test]
fn a_copy_passed_over_for_coming_round_is_read_once_the_name_it_came_round_to_is_found() {
    // names[0]'s first copy says its base, names[1], is of 2 bytes, so it
    // fails over it; its second names names[2]. names[1]'s first copy
    // names names[2], whose only copy names names[1]: a chain coming round
    // while names[1] is being found. names[1]'s second copy is the blob
    // "a". Once names[1] is found, names[2] reads over it, and names[0]'s
    // second copy over names[2].
    let names = names(3);
    let first = (
        names.clone(),
        vec![delta_over(names[1], 2), over(names[2]), over(names[1])],
    );
    let second = (names[..2].to_vec(), vec![over(names[2]), blob(b"a")]);
    let folders = packed_as(&[first, second]);
    let objects = folders.0[0].clone();
    let (read, _) = within(20, move || read_each(&objects, &names[..1])).expect("the read ended");
    assert_eq!(
        read,
        vec![Ok(Some(b"x".to_vec()))],
        "a copy passed over for coming round to a name was passed over once that name was found"
    );
}
