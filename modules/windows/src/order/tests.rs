use super::*;

/// The ids from which the tests' workspaces are empty, as the one niri
/// keeps last on each screen.
const EMPTY: u64 = 100;

type Screens<'a> = &'a [(&'a str, &'a [u64])];

/// niri's list of workspaces, each screen's given top to bottom by id,
/// each below `EMPTY` holding a window.
fn listed(screens: Screens) -> Vec<Value> {
    screens
        .iter()
        .flat_map(|(output, ids)| {
            ids.iter().enumerate().map(move |(at, id)| {
                let window = (*id < EMPTY).then_some(id + 1000);
                json!({"id": id, "idx": at + 1, "output": output, "active_window_id": window})
            })
        })
        .collect()
}

/// Each screen's workspaces top to bottom once they're put back in the
/// order given, each move made from the list as niri would give it then.
fn settled(kept: &[u64], screens: Screens) -> Vec<(String, Vec<u64>)> {
    let mut now: Vec<(String, Vec<u64>)> = screens
        .iter()
        .map(|(output, ids)| ((*output).to_owned(), ids.to_vec()))
        .collect();
    for _ in 0..MOVES {
        let shown: Vec<(&str, &[u64])> = now
            .iter()
            .map(|(output, ids)| (output.as_str(), ids.as_slice()))
            .collect();
        let Some(one) = next(kept, &listed(&shown)) else {
            return now;
        };
        let asked = &one["Action"]["MoveWorkspaceToIndex"];
        let id = asked["reference"]["Id"].as_u64().unwrap_or_default();
        let index = usize::try_from(asked["index"].as_u64().unwrap_or(1)).unwrap_or(1) - 1;
        for (_, ids) in &mut now {
            if let Some(at) = ids.iter().position(|each| *each == id) {
                ids.remove(at);
                ids.insert(index, id);
            }
        }
    }
    panic!("the workspaces were never in order: {now:?}");
}

fn order_of(screens: Screens, at: Instant) -> Order {
    let mut order = Order::default();
    let put = order.heard(&listed(screens), at);
    assert_eq!(put, None, "the first list was put back");
    order
}

/// What the order puts back once it hears these screens, or none.
fn after(order: &mut Order, screens: Screens, at: Instant) -> Option<Vec<(String, Vec<u64>)>> {
    order
        .heard(&listed(screens), at)
        .map(|kept| settled(&kept, screens))
}

/// The screens as a test expects them.
fn each(screens: Screens) -> Vec<(String, Vec<u64>)> {
    screens
        .iter()
        .map(|(output, ids)| ((*output).to_owned(), ids.to_vec()))
        .collect()
}

#[test]
fn a_screen_coming_back_takes_its_workspaces_in_the_order_joel_had() {
    let start = Instant::now();
    let mut order = order_of(&[("eDP-1", &[2, 22, 21, 19, 23, 20, 30, 100])], start);
    let back: Screens = &[
        ("DP-1", &[19, 21, 23, 20, 2, 22, 30, 101]),
        ("eDP-1", &[102]),
    ];
    assert_eq!(
        after(&mut order, back, start),
        Some(each(&[
            ("DP-1", &[2, 22, 21, 19, 23, 20, 30, 101]),
            ("eDP-1", &[102])
        ])),
        "the workspaces weren't put back in Joel's order"
    );
}

#[test]
fn putting_them_back_only_moves_a_workspace_on_its_own_screen() {
    let one = next(
        &[2, 19],
        &listed(&[("DP-1", &[19, 2, 101]), ("eDP-1", &[102])]),
    );
    assert!(
        one.as_ref()
            .and_then(|one| one["Action"].as_object())
            .is_some_and(|action| {
                action.len() == 1 && action.contains_key("MoveWorkspaceToIndex")
            }),
        "niri was asked more than to move a workspace, or nothing: {one:?}"
    );
}

#[test]
fn a_workspace_joel_moves_himself_stays_where_he_put_it() {
    let start = Instant::now();
    let mut order = order_of(&[("eDP-1", &[2, 19, 21, 100])], start);
    let his = after(&mut order, &[("eDP-1", &[21, 2, 19, 100])], start);
    let back: Screens = &[("DP-1", &[2, 19, 21, 101]), ("eDP-1", &[102])];
    assert_eq!(
        (his, after(&mut order, back, start)),
        (
            None,
            Some(each(&[("DP-1", &[21, 2, 19, 101]), ("eDP-1", &[102])]))
        ),
        "Joel's own order wasn't kept"
    );
}

#[test]
fn what_niri_lists_while_the_order_settles_isnt_kept_as_joels() {
    let start = Instant::now();
    let mut order = order_of(&[("eDP-1", &[2, 19, 21, 100])], start);
    let came = after(
        &mut order,
        &[("DP-1", &[21, 19, 2, 101]), ("eDP-1", &[102])],
        start,
    );
    let between = after(
        &mut order,
        &[("DP-1", &[2, 21, 19, 101]), ("eDP-1", &[102])],
        start,
    );
    let gone: Screens = &[("DP-1", &[19, 21, 2, 101])];
    assert_eq!(
        (
            came.is_some(),
            between,
            after(&mut order, gone, start + SETTLING)
        ),
        (true, None, Some(each(&[("DP-1", &[2, 19, 21, 101])]))),
        "an order niri gave while the order settled was taken for Joel's"
    );
}

#[test]
fn a_workspace_taken_home_while_the_order_settles_goes_to_its_place() {
    let start = Instant::now();
    let mut order = order_of(&[("eDP-1", &[5, 2, 19, 100])], start);
    let came = after(
        &mut order,
        &[("DP-1", &[2, 101]), ("eDP-1", &[5, 19, 100])],
        start,
    );
    let home: Screens = &[("DP-1", &[19, 2, 101]), ("eDP-1", &[5, 100])];
    let later = start + SETTLING / 2;
    assert_eq!(
        (came.is_some(), after(&mut order, home, later)),
        (
            true,
            Some(each(&[("DP-1", &[2, 19, 101]), ("eDP-1", &[5, 100])]))
        ),
        "a workspace the shell took home wasn't put in its place"
    );
}

#[test]
fn the_order_settles_until_the_workspaces_rest() {
    let start = Instant::now();
    let mut order = order_of(&[("eDP-1", &[5, 2, 19, 100])], start);
    let came = after(
        &mut order,
        &[("DP-1", &[2, 101]), ("eDP-1", &[5, 19, 100])],
        start,
    );
    let changed = start + SETTLING * 4 / 5;
    let still = after(
        &mut order,
        &[("DP-1", &[2, 101]), ("eDP-1", &[19, 5, 100])],
        changed,
    );
    let home: Screens = &[("DP-1", &[19, 2, 101]), ("eDP-1", &[5, 100])];
    assert_eq!(
        (
            came.is_some(),
            still,
            after(&mut order, home, start + SETTLING)
        ),
        (
            true,
            None,
            Some(each(&[("DP-1", &[2, 19, 101]), ("eDP-1", &[5, 100])]))
        ),
        "the order stopped settling while the workspaces still changed"
    );
}

#[test]
fn a_workspace_joel_takes_to_another_screen_stays_where_he_put_it() {
    let start = Instant::now();
    let mut order = order_of(&[("DP-1", &[2, 19, 101]), ("eDP-1", &[5, 100])], start);
    let his = after(
        &mut order,
        &[("DP-1", &[5, 2, 19, 101]), ("eDP-1", &[100])],
        start,
    );
    assert_eq!(his, None, "a workspace Joel moved was put back");
}

#[test]
fn a_screen_going_puts_the_monitors_workspaces_before_the_laptops() {
    let start = Instant::now();
    let mut order = order_of(&[("DP-1", &[2, 19, 100]), ("eDP-1", &[5, 101])], start);
    assert_eq!(
        after(&mut order, &[("eDP-1", &[5, 2, 19, 101])], start),
        Some(each(&[("eDP-1", &[2, 19, 5, 101])])),
        "the monitor's workspaces didn't lead"
    );
}

#[test]
fn a_screen_going_again_gives_back_the_order_joel_had_before_it_came() {
    let start = Instant::now();
    let mut order = order_of(&[("eDP-1", &[5, 2, 19, 100])], start);
    let came: Screens = &[("DP-1", &[2, 19, 101]), ("eDP-1", &[5, 100])];
    let first = after(&mut order, came, start);
    let settled_then = after(&mut order, came, start + SETTLING);
    let gone: Screens = &[("eDP-1", &[5, 100, 2, 19])];
    assert_eq!(
        (
            first,
            settled_then,
            after(&mut order, gone, start + SETTLING)
        ),
        (
            Some(each(came)),
            None,
            Some(each(&[("eDP-1", &[5, 2, 19, 100])]))
        ),
        "the two screens' workspaces didn't keep how they interleaved"
    );
}

#[test]
fn a_workspace_the_order_doesnt_hold_stays_after_those_it_does() {
    let start = Instant::now();
    let mut order = order_of(&[("eDP-1", &[2, 19, 100])], start);
    assert_eq!(
        after(
            &mut order,
            &[("DP-1", &[40, 19, 2]), ("eDP-1", &[101])],
            start
        ),
        Some(each(&[("DP-1", &[2, 19, 40]), ("eDP-1", &[101])])),
        "a workspace niri made was put among Joel's"
    );
}

#[test]
fn the_screens_staying_as_they_are_puts_nothing_back() {
    let start = Instant::now();
    let mut order = order_of(&[("DP-1", &[2, 19, 100]), ("eDP-1", &[101])], start);
    let his = after(
        &mut order,
        &[("DP-1", &[19, 2, 100]), ("eDP-1", &[101])],
        start,
    );
    assert_eq!(his, None, "a move of Joel's was undone");
}
