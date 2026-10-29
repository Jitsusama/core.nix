//! How a call to Drive is made again: after a wait while Drive says it is
//! busy, and once with a fresh token when Drive stops taking the one held.

use std::thread;
use std::time::Duration;

use crate::drive::Fault;
use crate::token::Login;

/// A call made again after each of `waits` while Drive answers it busy.
pub fn patiently<T>(
    waits: &[Duration],
    mut call: impl FnMut() -> Result<T, Fault>,
) -> Result<T, Fault> {
    let mut waits = waits.iter();
    loop {
        match call() {
            Err(Fault::Busy(why)) => match waits.next() {
                Some(wait) => thread::sleep(*wait),
                None => return Err(Fault::Busy(why)),
            },
            other => return other,
        }
    }
}

/// A call to Drive with a token, made once more with a fresh token when
/// Drive says the one it had stopped counting.
pub fn again<T>(
    login: &mut Login,
    agent: &ureq::Agent,
    call: impl Fn(&str) -> Result<T, Fault>,
) -> Result<T, Fault> {
    match call(&login.token(agent).map_err(Fault::Other)?) {
        Err(Fault::Unauthorized) => {
            login.forget();
            call(&login.token(agent).map_err(Fault::Other)?)
        }
        other => other,
    }
}
