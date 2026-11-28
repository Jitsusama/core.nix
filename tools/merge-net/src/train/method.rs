//! How GitHub merges a rider's pull request once its candidate passes: as
//! the merge its asker named, or with a merge commit when none was named.
//! On 28 November #7383's owner asked for a squash and the train merged it
//! with a merge commit all the same, since it always asked GitHub for one.
//! The review tools name the method in `REVIEW_MERGE_METHOD` as they ask
//! `ready`, and the rider keeps it, so the train merges it so. Both leave
//! main the tree the candidate tested, since GitHub's squash applies the
//! same change to the same main as its merge does, and the train checks
//! that tree after the merges as it always has. A rebase isn't taken:
//! GitHub gives its commits their own subjects, so main would hold no
//! subject naming the pull request's number, which the net's searches and
//! reverts read it from.

use std::env;

/// The variable the review tools name the method in, unset when the asker
/// named none.
pub const ASKED: &str = "REVIEW_MERGE_METHOD";

/// How GitHub merges a pull request, as its API names each.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Method {
    /// A merge commit of two parents, the head its second.
    #[default]
    Merge,
    /// One commit of the change, on main's head alone.
    Squash,
}

impl Method {
    /// The method `word` names, as GitHub's API words it; none for any
    /// other word, a rebase's included.
    #[must_use]
    pub fn named(word: &str) -> Option<Method> {
        match word.trim() {
            "merge" => Some(Method::Merge),
            "squash" => Some(Method::Squash),
            _ => None,
        }
    }

    /// How GitHub's API words it.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Method::Merge => "merge",
            Method::Squash => "squash",
        }
    }

    /// The method this run of `ready` was asked for; a merge where none
    /// was named.
    ///
    /// # Errors
    ///
    /// The word given when it names no method the train merges by.
    pub fn asked() -> Result<Method, String> {
        match env::var(ASKED) {
            Ok(word) if !word.trim().is_empty() => Method::named(&word).ok_or(word),
            _ => Ok(Method::Merge),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Method;

    #[test]
    fn each_method_is_read_back_from_its_own_word() {
        for method in [Method::Merge, Method::Squash] {
            assert_eq!(
                Method::named(method.word()),
                Some(method),
                "{method:?} reads back"
            );
        }
    }

    #[test]
    fn a_word_the_train_merges_by_no_method_of_names_none() {
        assert_eq!(
            Method::named("fast-forward"),
            None,
            "an unknown word names none"
        );
        assert_eq!(Method::named("rebase"), None, "a rebase names none");
        assert_eq!(Method::named(""), None, "an empty word names none");
    }

    #[test]
    fn a_rider_that_named_nothing_merges_with_a_merge_commit() {
        assert_eq!(
            Method::default(),
            Method::Merge,
            "a merge commit by default"
        );
    }
}
