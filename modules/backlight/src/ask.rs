//! Asking another program over the bus without waiting on it forever.
//!
//! The daemon does all its work in one loop, so a call left unanswered holds
//! the brightness, Joel's keys and the lid along with it. wl-gammarelay-rs,
//! when the bus starts it for the first warmth set, leaves that set
//! unanswered until another message reaches it. So each call gets a moment
//! to answer; one that doesn't is sent once more, which wakes a program in
//! that state, and after that it's given up on.

use std::future::Future;
use std::time::Duration;

/// How long a call is given to answer before it's sent again.
pub const MOMENT: Duration = Duration::from_millis(500);

/// Asks, and asks once more when no answer comes within `wait`. None when
/// neither is answered in time.
pub async fn twice<T, F, A>(wait: Duration, mut ask: A) -> Option<T>
where
    A: FnMut() -> F,
    F: Future<Output = T>,
{
    for _ in 0..2 {
        if let Ok(answer) = tokio::time::timeout(wait, ask()).await {
            return Some(answer);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::cell::Cell;
    use std::future;

    const WAIT: Duration = Duration::from_millis(10);

    #[tokio::test]
    async fn an_answer_in_time_is_asked_for_once() {
        let asked = Cell::new(0);
        let answer = twice(WAIT, || {
            asked.set(asked.get() + 1);
            future::ready(7)
        })
        .await;
        assert_eq!(answer, Some(7));
        assert_eq!(asked.get(), 1);
    }

    #[tokio::test]
    async fn a_call_left_unanswered_is_sent_again() {
        let asked = Cell::new(0);
        let answer = twice(WAIT, || {
            asked.set(asked.get() + 1);
            let first = asked.get() == 1;
            async move {
                if first {
                    future::pending::<()>().await;
                }
                7
            }
        })
        .await;
        assert_eq!(answer, Some(7), "the second ask wasn't sent or heard");
        assert_eq!(asked.get(), 2);
    }

    #[tokio::test]
    async fn a_program_that_never_answers_is_given_up_on() {
        let asked = Cell::new(0);
        let answer = twice(WAIT, || {
            asked.set(asked.get() + 1);
            future::pending::<u8>()
        })
        .await;
        assert_eq!(answer, None);
        assert_eq!(asked.get(), 2);
    }
}
