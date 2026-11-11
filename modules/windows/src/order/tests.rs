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
        moved(&mut now, &one);
    }
    panic!("the workspaces were never in order: {now:?}");
}

/// Each screen's workspaces as niri has them once it makes this move.
fn moved(now: &mut [(String, Vec<u64>)], one: &Value) {
    let asked = &one["Action"]["MoveWorkspaceToIndex"];
    let id = asked["reference"]["Id"].as_u64().unwrap_or_default();
    let index = usize::try_from(asked["index"].as_u64().unwrap_or(1)).unwrap_or(1) - 1;
    for (_, ids) in now {
        if let Some(at) = ids.iter().position(|each| *each == id) {
            ids.remove(at);
            ids.insert(index, id);
        }
    }
}

/// What a stand-in niri holds: each screen's workspaces, the window
/// focused, a window Joel presses once the first move is made, and each
/// window it was asked to focus.
#[derive(Default)]
struct Held {
    screens: Vec<(String, Vec<u64>)>,
    focused: Option<u64>,
    pressed: Option<u64>,
    focusing: Vec<u64>,
}

/// A stand-in for niri's socket, answering the asks a putting back makes,
/// one a connection, as niri does, from what it holds.
struct Niri {
    socket: PathBuf,
    held: std::sync::Arc<std::sync::Mutex<Held>>,
}

impl Niri {
    fn new(test: &str, screens: Screens, focused: Option<u64>) -> Niri {
        let socket = std::env::temp_dir().join(format!("order-{test}-{}", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listening = std::os::unix::net::UnixListener::bind(&socket).expect("niri's socket");
        let held = std::sync::Arc::new(std::sync::Mutex::new(Held {
            screens: each(screens),
            focused,
            ..Held::default()
        }));
        let serving = held.clone();
        std::thread::spawn(move || {
            for stream in listening.incoming().map_while(Result::ok) {
                let mut said = String::new();
                let mut lines = BufReader::new(&stream);
                if lines.read_line(&mut said).is_err() {
                    continue;
                }
                let asked: Value = serde_json::from_str(&said).unwrap_or_default();
                let answer = Niri::answer(&mut serving.lock().expect("niri's state"), &asked);
                let _ = writeln!(&stream, "{answer}");
            }
        });
        Niri { socket, held }
    }

    fn answer(held: &mut Held, asked: &Value) -> Value {
        if asked == "Workspaces" {
            let shown: Vec<(&str, &[u64])> = held
                .screens
                .iter()
                .map(|(output, ids)| (output.as_str(), ids.as_slice()))
                .collect();
            return json!({"Ok": {"Workspaces": listed(&shown)}});
        }
        if asked == "FocusedWindow" {
            return json!({"Ok": {"FocusedWindow": held.focused.map(|id| json!({"id": id}))}});
        }
        if let Some(id) = asked["Action"]["FocusWindow"]["id"].as_u64() {
            held.focusing.push(id);
            held.focused = Some(id);
        } else {
            moved(&mut held.screens, asked);
            if let Some(window) = held.pressed.take() {
                held.focused = Some(window);
            }
        }
        json!({"Ok": "Handled"})
    }

    /// What it holds once a putting back of `kept`, to focus `focus`
    /// again, is done, while it's the newest unless `overtaken`.
    fn after(&self, kept: &[u64], focus: Option<u64>, overtaken: bool) -> (Option<u64>, Vec<u64>) {
        let newest = AtomicU64::new(1);
        let putting = Putting {
            kept: kept.to_vec(),
            focus,
        };
        if overtaken {
            newest.store(2, Ordering::SeqCst);
        }
        putting_back(&self.socket, &putting, &newest, 1);
        let held = self.held.lock().expect("niri's state");
        (held.focused, held.focusing.clone())
    }
}

impl Drop for Niri {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket);
    }
}

fn order_of(screens: Screens, at: Instant) -> Order {
    let mut order = Order::default();
    let put = order.heard(&listed(screens), None, at);
    assert_eq!(put, None, "the first list was put back");
    order
}

/// What the order puts back once it hears these screens, or none.
fn after(order: &mut Order, screens: Screens, at: Instant) -> Option<Vec<(String, Vec<u64>)>> {
    order
        .heard(&listed(screens), None, at)
        .map(|put| settled(&put.kept, screens))
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
fn a_screen_coming_back_focuses_again_the_window_joel_had() {
    let start = Instant::now();
    let mut order = order_of(&[("eDP-1", &[2, 19, 100])], start);
    let back = order.heard(
        &listed(&[("DP-1", &[19, 2, 101]), ("eDP-1", &[102])]),
        Some(1019),
        start,
    );
    assert_eq!(
        back.and_then(|put| put.focus),
        Some(1019),
        "the window Joel had focused wasn't to be focused again"
    );
}

#[test]
fn a_window_joel_focuses_while_the_order_settles_stays_focused() {
    let start = Instant::now();
    let mut order = order_of(&[("eDP-1", &[5, 2, 19, 100])], start);
    let came = order.heard(
        &listed(&[("DP-1", &[2, 101]), ("eDP-1", &[5, 19, 100])]),
        Some(1019),
        start,
    );
    let home = order.heard(
        &listed(&[("DP-1", &[19, 2, 101]), ("eDP-1", &[5, 100])]),
        Some(1005),
        start + SETTLING / 2,
    );
    assert_eq!(
        (came.and_then(|put| put.focus), home.map(|put| put.focus)),
        (Some(1019), Some(None)),
        "a window was focused again other than as the screens changed"
    );
}

#[test]
fn the_window_joel_had_is_focused_again_once_his_workspaces_are_back() {
    let niri = Niri::new(
        "refocus",
        &[("DP-1", &[19, 2, 101]), ("eDP-1", &[102])],
        Some(1019),
    );
    assert_eq!(
        niri.after(&[2, 19], Some(1002), false),
        (Some(1002), vec![1002]),
        "the window Joel had wasn't focused again"
    );
}

#[test]
fn a_window_joel_focuses_between_the_moves_stays_focused() {
    let niri = Niri::new("pressed", &[("DP-1", &[19, 5, 2, 101])], Some(1019));
    niri.held.lock().expect("niri's state").pressed = Some(1005);
    assert_eq!(
        niri.after(&[2, 5, 19], Some(1002), false),
        (Some(1005), vec![]),
        "a window Joel focused during the moves was undone"
    );
}

#[test]
fn a_putting_back_a_newer_one_took_over_focuses_nothing() {
    let niri = Niri::new("overtaken", &[("DP-1", &[19, 2, 101])], Some(1019));
    assert_eq!(
        niri.after(&[2, 19], Some(1002), true),
        (Some(1019), vec![]),
        "an older putting back focused a window after a newer one"
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
