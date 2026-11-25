//! Origin's main, fetched afresh beside other fetches of the same checkout,
//! for `ready` and for the train's landing alike.

use std::thread;
use std::time::Duration;

/// How many times `main` fetches before it gives the refusal.
const FETCHES: usize = 3;

/// How long `main` waits after a refusal before fetching again, so the
/// fetch holding the ref's lock has time to finish; none in tests, whose
/// stand-in answers at once.
const PAUSE: Duration = if cfg!(test) {
    Duration::ZERO
} else {
    Duration::from_secs(2)
};

/// Origin's main as it stands, fetched afresh through `git`. Every tree
/// cut from one checkout shares its `refs/remotes/origin/main`, so a fetch
/// in one tree while another fetches finds the ref locked, or moved just
/// before it could move it, and git refuses ("cannot lock ref"), as
/// #6509's and #6491's readies were on 25 November. The other fetch leaves
/// the ref at main, so fetching again a moment later lands; any other
/// failure is said at once.
pub fn main(git: &dyn Fn(&[&str]) -> Result<String, String>) -> Result<String, String> {
    let mut refused = String::new();
    for fetch in 0..FETCHES {
        if fetch > 0 {
            thread::sleep(PAUSE);
        }
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
        let fetches = [Err(raced), Ok("")];
        let asked = Cell::new(0);
        let git = fetching(&fetches, &asked);
        assert_eq!(main(&git), Ok("main".to_owned()), "the second fetch lands");
        assert_eq!(asked.get(), 2, "it fetched once more after the race");
    }

    #[test]
    fn a_fetch_that_keeps_racing_says_the_last_refusal() {
        let raced = "error: cannot lock ref 'refs/remotes/origin/main': File exists.";
        let fetches = [Err(raced), Err(raced), Err(raced)];
        let asked = Cell::new(0);
        let git = fetching(&fetches, &asked);
        assert_eq!(main(&git), Err(raced.to_owned()), "the refusal is said");
        assert_eq!(asked.get(), FETCHES, "it gave up after {FETCHES} fetches");
    }

    #[test]
    fn any_other_failed_fetch_is_said_at_once() {
        let fetches = [Err("fatal: unable to access"), Ok("")];
        let asked = Cell::new(0);
        let git = fetching(&fetches, &asked);
        assert_eq!(
            main(&git),
            Err("fatal: unable to access".to_owned()),
            "a lost network isn't a race"
        );
        assert_eq!(asked.get(), 1, "it didn't fetch again");
    }
}
