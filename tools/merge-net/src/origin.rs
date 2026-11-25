//! Origin's main, fetched afresh beside other fetches of the same checkout.

/// How many times `main` fetches before it gives the refusal.
const FETCHES: usize = 3;

/// Origin's main as it stands, fetched afresh through `git`. Every tree
/// cut from one checkout shares its `refs/remotes/origin/main`, so a ready
/// asked in one tree while another fetches can read the ref just before
/// the other moves it, and git refuses to move it again ("cannot lock
/// ref"), as #6509's ready was on 25 November. The other fetch has left
/// the ref at main by then, so fetching again lands; any other failure is
/// said at once.
pub fn main(git: &dyn Fn(&[&str]) -> Result<String, String>) -> Result<String, String> {
    let mut refused = String::new();
    for _ in 0..FETCHES {
        match git(&["fetch", "--quiet", "origin", "main"]) {
            Ok(_) => return git(&["rev-parse", "refs/remotes/origin/main"]),
            Err(why) if why.contains("cannot lock ref") => refused = why,
            Err(why) => return Err(why),
        }
    }
    Err(refused)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    /// A git that answers each fetch from `fetches` in turn, and every
    /// other call with main's commit, counting the fetches in `asked`.
    fn fetching<'a>(
        fetches: &'a [Result<&'a str, &'a str>],
        asked: &'a Cell<usize>,
    ) -> impl Fn(&[&str]) -> Result<String, String> + 'a {
        move |arguments: &[&str]| {
            if arguments.first() != Some(&"fetch") {
                return Ok("main".to_owned());
            }
            let fetch = fetches[asked.get()];
            asked.set(asked.get() + 1);
            fetch.map(str::to_owned).map_err(str::to_owned)
        }
    }

    #[test]
    fn a_fetch_beside_another_is_fetched_again() {
        let raced =
            "error: cannot lock ref 'refs/remotes/origin/main': is at f80b but expected 9bb9";
        let asked = Cell::new(0);
        let git = fetching(&[Err(raced), Ok("")], &asked);
        assert_eq!(main(&git), Ok("main".to_owned()), "the second fetch lands");
        assert_eq!(asked.get(), 2, "it fetched once more after the race");
    }

    #[test]
    fn a_fetch_that_keeps_racing_says_the_last_refusal() {
        let raced = "error: cannot lock ref 'refs/remotes/origin/main'";
        let asked = Cell::new(0);
        let git = fetching(&[Err(raced), Err(raced), Err(raced)], &asked);
        assert_eq!(main(&git), Err(raced.to_owned()), "the refusal is said");
        assert_eq!(asked.get(), FETCHES, "it gave up after {FETCHES} fetches");
    }

    #[test]
    fn any_other_failed_fetch_is_said_at_once() {
        let asked = Cell::new(0);
        let git = fetching(&[Err("fatal: unable to access"), Ok("")], &asked);
        assert_eq!(
            main(&git),
            Err("fatal: unable to access".to_owned()),
            "a lost network isn't a race"
        );
        assert_eq!(asked.get(), 1, "it didn't fetch again");
    }
}
