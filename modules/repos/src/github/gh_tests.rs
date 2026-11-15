//! The narrow reader gh's sign-in is asked through: one host, no
//! redirects, queries alone, a token said nowhere, and a refusal that
//! stops rather than asking again or another way.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::thread;

use super::tests::{answer, one};
use super::*;
use crate::gh::tests::{PLANTED, gh};
use crate::gh::token;

fn planted() -> Result<Token, String> {
    token(gh(&format!("printf '{PLANTED}\\n'")))
}

#[test]
fn only_github_s_graphql_endpoint_is_asked() {
    let token = planted().expect("a token");
    for endpoint in [
        "https://api.github.com.example.org/graphql",
        "http://api.github.com/graphql",
        "https://api.github.com/graphql/../repos",
        "https://example.org/graphql",
    ] {
        let failed = post(endpoint, &token, "query { viewer { login } }").expect_err("refused");
        assert!(failed.refused, "asking {endpoint} could be tried again");
        assert_eq!(
            failed.why,
            format!("repos asks only {GRAPHQL}, not {endpoint}"),
            "{endpoint} wasn't refused"
        );
    }
}

#[test]
fn a_document_that_writes_or_listens_is_refused_before_it_goes() {
    let token = planted().expect("a token");
    for document in [
        "mutation { addStar(input: {starrableId: \"x\"}) { clientMutationId } }",
        "subscription { thing }",
        "query { viewer { login } }\nmutation m { thing }",
        "{ viewer { login } }",
    ] {
        let failed = post(GRAPHQL, &token, document).expect_err("refused");
        assert!(failed.refused, "{document} could be tried again");
        assert_eq!(
            failed.why, "repos sends GitHub one query, nothing that writes or listens",
            "{document} wasn't refused"
        );
    }
}

#[test]
fn a_query_naming_a_repository_after_a_writing_word_still_reads() {
    let asked = [Asked {
        key: "github:mutation/subscription".to_owned(),
        owner: "mutation".to_owned(),
        name: "subscription".to_owned(),
    }];
    assert_eq!(reads_only(&query(&asked)), Ok(()));
    assert_eq!(reads_only("# mutation\nquery { viewer { login } }"), Ok(()));
}

#[test]
fn a_redirect_is_refused_not_followed() {
    let elsewhere = TcpListener::bind("127.0.0.1:0").expect("a second listener");
    elsewhere
        .set_nonblocking(true)
        .expect("it waits for no one");
    let there = elsewhere.local_addr().expect("its address");
    let here = TcpListener::bind("127.0.0.1:0").expect("a listener");
    let endpoint = format!("http://{}/graphql", here.local_addr().expect("its address"));
    let answered = thread::spawn(move || {
        let (mut caller, _) = here.accept().expect("a caller");
        let mut asked = [0; 4096];
        let _ = caller.read(&mut asked).expect("the caller asks");
        let redirect = format!(
            "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://{there}/graphql\r\n\
             Content-Length: 0\r\nConnection: close\r\n\r\n"
        );
        caller.write_all(redirect.as_bytes()).expect("it answers");
    });
    let failed = send(&endpoint, &planted().expect("a token"), "query { a }").expect_err("refused");
    answered.join().expect("the stand-in ends");
    assert!(failed.refused, "a redirect could be tried again");
    assert_eq!(
        failed.why, "GitHub answered with a redirect (307), which repos doesn't follow",
        "the redirect wasn't refused"
    );
    assert!(
        elsewhere.accept().is_err(),
        "the redirect was followed to where it pointed"
    );
}

#[test]
fn a_sign_in_gh_lacks_marks_each_listed_one_unreadable_and_asks_nothing() {
    let mut asked_github = 0;
    let seen = refresh_through_gh(
        &[one("someone/private"), one("someone/other")],
        7,
        || Err("gh has no sign-in for github.com (it exited 1)".to_owned()),
        |_, _| {
            asked_github += 1;
            Ok(Value::Null)
        },
    );
    assert_eq!(asked_github, 0, "GitHub was asked without a token");
    assert_eq!(
        seen.unread,
        [
            "github:someone/private: not readable here: gh has no sign-in for github.com \
             (it exited 1)",
            "github:someone/other: not readable here: gh has no sign-in for github.com \
             (it exited 1)",
        ]
    );
}

#[test]
fn a_refused_sign_in_stops_at_the_first_query_and_says_why_for_each() {
    let many: Vec<Asked> = (0..=PER_QUERY)
        .map(|at| one(&format!("someone/r{at}")))
        .collect();
    let mut asked_github = 0;
    let seen = refresh_through_gh(&many, 7, planted, |_, _| {
        asked_github += 1;
        Err(Failed::refused(
            "GitHub refused the sign-in (401 Unauthorized): Bad credentials".to_owned(),
        ))
    });
    assert_eq!(asked_github, 1, "a refused sign-in was asked again");
    assert_eq!(
        seen.unread.len(),
        many.len(),
        "not every one was said: {seen:?}"
    );
    assert_eq!(
        seen.unread[0],
        "github:someone/r0: not readable here: GitHub refused the sign-in (401 Unauthorized): \
         Bad credentials"
    );
}

#[test]
fn a_query_github_gives_up_on_marks_that_one_alone_and_goes_on() {
    let answer = answer();
    let mut asked_github = 0;
    let seen = refresh_through_gh(
        &[one("someone/busy"), one("Jitsusama/core.nix")],
        7,
        planted,
        |_, _| {
            asked_github += 1;
            if asked_github == 1 {
                Err("GitHub gave up on the query (504 Gateway Timeout)"
                    .to_owned()
                    .into())
            } else {
                Ok(answer.clone())
            }
        },
    );
    assert_eq!(asked_github, 2, "the next one wasn't asked after");
    assert_eq!(
        seen.unread[0],
        "github:someone/busy: not readable here: GitHub gave up on the query (504 Gateway Timeout)"
    );
    assert_eq!(seen.changes["github:Jitsusama/core.nix"].len(), 3);
}

#[test]
fn what_gh_s_sign_in_read_is_kept_unreadable_by_others_and_without_its_token() {
    let asked = [one("Jitsusama/core.nix")];
    let answer = answer();
    let mut sent = Vec::new();
    let seen = refresh_through_gh(&asked, 7, planted, |token, query| {
        sent.push(token.header());
        assert_eq!(reads_only(query), Ok(()), "the query writes");
        Ok(answer.clone())
    });
    assert_eq!(
        sent,
        [format!("bearer {PLANTED}")],
        "the token wasn't sent once"
    );
    assert_eq!(seen.through["github:Jitsusama/core.nix"], Through::Gh);
    let file = std::env::temp_dir()
        .join(format!("repos-{}-kept", std::process::id()))
        .join("github.json");
    keep(&file, &seen).expect("it keeps");
    let kept = std::fs::read_to_string(&file).expect("it reads back");
    let mode = std::fs::metadata(&file)
        .expect("it's there")
        .permissions()
        .mode();
    let _ = std::fs::remove_dir_all(file.parent().expect("its folder"));
    assert_eq!(mode & 0o777, 0o600, "others can read what was kept");
    assert!(
        kept.contains(r#""through":{"github:Jitsusama/core.nix":"gh"}"#),
        "{kept}"
    );
    let said = format!(
        "{seen:?}{}",
        serde_json::to_string(&seen).expect("it writes")
    );
    for found in [&kept, &said] {
        assert!(
            !found.contains(PLANTED),
            "the token was kept or said: {found}"
        );
    }
}
