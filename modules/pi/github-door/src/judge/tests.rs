//! The judge's cases: what passes, what is refused, and the word and
//! route each refusal says.

use super::*;

fn judged(method: &str, host: Host, target: &str) -> Result<Seen, Refusal> {
    judge(
        &Call {
            method,
            host,
            target,
            body: b"",
        },
        &[],
    )
}

fn why(method: &str, host: Host, target: &str) -> Option<Why> {
    judged(method, host, target)
        .err()
        .map(|refusal| refusal.why)
}

fn passes(method: &str, host: Host, target: &str) {
    assert_eq!(why(method, host, target), None, "{method} {target}");
}

#[test]
fn the_four_hosts_are_named_and_nothing_else() {
    assert_eq!(Host::named("api.github.com"), Some(Host::Api));
    assert_eq!(Host::named("GitHub.com."), Some(Host::Web));
    assert_eq!(Host::named("gist.github.com"), None);
    assert_eq!(Host::named("api.github.com.evil"), None);
}

#[test]
fn every_get_passes_and_says_its_path_without_its_query() {
    assert_eq!(
        judged("GET", Host::Api, "/repos/o/r/issues?state=open&token=x"),
        Ok(Seen::Path("/repos/o/r/issues".to_owned()))
    );
    passes("GET", Host::Api, "/user");
    passes("HEAD", Host::Api, "/rate_limit");
    passes("GET", Host::Uploads, "/repos/o/r/releases/1/assets");
    passes("GET", Host::Codeload, "/o/r/tar.gz/main");
    passes(
        "GET",
        Host::Web,
        "/o/r.git/info/refs?service=git-upload-pack",
    );
}

#[test]
fn writes_to_issues_comments_labels_and_reactions_pass() {
    passes("POST", Host::Api, "/repos/o/r/issues");
    passes("PATCH", Host::Api, "/repos/o/r/issues/4");
    passes("POST", Host::Api, "/repos/o/r/issues/4/comments");
    passes("PATCH", Host::Api, "/repos/o/r/issues/comments/9");
    passes("DELETE", Host::Api, "/repos/o/r/issues/comments/9");
    passes("POST", Host::Api, "/repos/o/r/issues/4/labels");
    passes("DELETE", Host::Api, "/repos/o/r/issues/4/labels/bug");
    passes("POST", Host::Api, "/repos/o/r/issues/4/sub_issues");
    passes("DELETE", Host::Api, "/repos/o/r/issues/4/sub_issue");
    passes("POST", Host::Api, "/repos/o/r/labels");
    passes("PATCH", Host::Api, "/repos/o/r/labels/bug");
    assert_eq!(
        why("DELETE", Host::Api, "/repos/o/r/labels/bug"),
        Some(Why::NotAllowed),
        "a repository's label deleted would leave every issue"
    );
    passes("POST", Host::Api, "/repos/o/r/issues/4/reactions");
    passes(
        "DELETE",
        Host::Api,
        "/repos/o/r/issues/comments/9/reactions/3",
    );
    passes("POST", Host::Api, "/repos/o/r/comments/9/reactions");
    passes("POST", Host::Api, "/repositories/123/issues/4/comments");
}

#[test]
fn writes_to_pulls_reviews_and_review_comments_pass() {
    passes("PATCH", Host::Api, "/repos/o/r/pulls/4");
    passes("POST", Host::Api, "/repos/o/r/pulls/4/requested_reviewers");
    passes("POST", Host::Api, "/repos/o/r/pulls/4/reviews");
    passes("PUT", Host::Api, "/repos/o/r/pulls/4/reviews/7");
    passes("DELETE", Host::Api, "/repos/o/r/pulls/4/reviews/7");
    passes("POST", Host::Api, "/repos/o/r/pulls/4/comments");
    passes("POST", Host::Api, "/repos/o/r/pulls/4/comments/8/replies");
    passes("PATCH", Host::Api, "/repos/o/r/pulls/comments/8");
    passes("DELETE", Host::Api, "/repos/o/r/pulls/comments/8");
    passes("POST", Host::Api, "/repos/o/r/pulls/comments/8/reactions");
}

fn reviewed(target: &str, body: &[u8]) -> Option<Why> {
    judge(
        &Call {
            method: "POST",
            host: Host::Api,
            target,
            body,
        },
        &[],
    )
    .err()
    .map(|refusal| refusal.why)
}

#[test]
fn a_pending_or_commenting_review_passes() {
    for target in [
        "/repos/o/r/pulls/4/reviews",
        "/repos/o/r/pulls/4/reviews/7/events",
    ] {
        for body in [
            &b""[..],
            br#"{"body": "approve of the names"}"#,
            br#"{"event": "COMMENT", "body": "x"}"#,
            br#"{"event": "comment"}"#,
        ] {
            assert_eq!(
                reviewed(target, body),
                None,
                "{target} {}",
                String::from_utf8_lossy(body)
            );
        }
    }
}

#[test]
fn a_review_approving_requesting_changes_or_dismissed_is_refused_with_its_route() {
    for target in [
        "/repos/o/r/pulls/4/reviews",
        "/repos/o/r/pulls/4/reviews/7/events",
        "/repositories/1/pulls/4/Reviews",
    ] {
        for body in [
            &br#"{"event": "APPROVE"}"#[..],
            br#"{"event": "request_changes", "body": "x"}"#,
            br#"{"event": "COMMENT", "event": "APPROVE"}"#,
            br#"{"\u0065vent": "APPROVE"}"#,
            br#"{"event": null}"#,
            br#"{"event": ["APPROVE"]}"#,
        ] {
            assert_eq!(
                reviewed(target, body),
                Some(Why::Verdict),
                "{target} {}",
                String::from_utf8_lossy(body)
            );
        }
    }
    let dismissed =
        judged("PUT", Host::Api, "/repos/o/r/pulls/4/reviews/7/dismissals").expect_err("refused");
    assert_eq!(dismissed.why, Why::Verdict);
    assert!(
        dismissed.detail.contains("review_draft"),
        "a dismissal names the review tools"
    );
    assert_eq!(
        reviewed("/repos/o/r/pulls/4/reviews?event=APPROVE", b""),
        Some(Why::Verdict)
    );
}

#[test]
fn a_review_whose_body_cant_be_read_is_refused() {
    for body in [
        &b"event=APPROVE"[..],
        br#"[{"event": "APPROVE"}]"#,
        br#"{"event": "COMMENT"} {"event": "APPROVE"}"#,
        b"\x1f\x8b\x08",
    ] {
        assert_eq!(
            reviewed("/repos/o/r/pulls/4/reviews", body),
            Some(Why::Unreadable),
            "{}",
            String::from_utf8_lossy(body)
        );
    }
    assert_eq!(
        why("PATCH", Host::Api, "/repos/o/r/pulls/4/reviews"),
        Some(Why::NotAllowed)
    );
}

#[test]
fn opening_or_merging_a_pull_request_is_refused_with_its_route() {
    let open = judged("POST", Host::Api, "/repos/o/r/pulls").expect_err("refused");
    assert_eq!(open.why, Why::PullCreate);
    assert!(
        open.detail.contains("review_offer propose"),
        "opening a pull request names review_offer propose"
    );
    let merge = judged("PUT", Host::Api, "/repos/o/r/pulls/4/merge").expect_err("refused");
    assert_eq!(merge.why, Why::Merge);
    assert!(
        merge.detail.contains("review_offer merge"),
        "a merge names review_offer merge"
    );
    assert_eq!(
        why("POST", Host::Api, "/repos/o/r/merges"),
        Some(Why::Merge)
    );
    assert_eq!(
        why("PUT", Host::Api, "/repos/o/r/pulls/4/update-branch"),
        Some(Why::NotAllowed)
    );
}

#[test]
fn credentials_are_refused_even_to_read() {
    for target in [
        "/user/keys",
        "/user/gpg_keys",
        "/user/installations",
        "/user/codespaces/secrets",
        "/users/someone/keys",
        "/repos/o/r/keys",
        "/repos/o/r/hooks",
        "/repos/o/r/hooks/1/deliveries",
        "/repos/o/r/installation",
        "/repos/o/r/actions/secrets",
        "/repos/o/r/actions/secrets/public-key",
        "/repos/o/r/actions/variables/NAME",
        "/repos/o/r/actions/runners/registration-token",
        "/repos/o/r/dependabot/secrets",
        "/repos/o/r/environments/prod/secrets",
        "/repos/o/r/environments/prod/variables",
        "/repositories/123/hooks",
        "/orgs/o/hooks",
        "/orgs/o/actions/secrets",
        "/orgs/o/personal-access-tokens",
        "/orgs/o/installations",
        "/organizations/5/hooks",
        "/authorizations",
        "/applications/abc/token",
        "/app/installations/1/access_tokens",
        "/installation/repositories",
    ] {
        assert_eq!(
            why("GET", Host::Api, target),
            Some(Why::Credentials),
            "{target}"
        );
        assert_eq!(
            why("POST", Host::Api, target),
            Some(Why::Credentials),
            "{target}"
        );
    }
    assert_eq!(
        why("GET", Host::Api, "/repos/o/r/HOOKS"),
        Some(Why::Credentials)
    );
    assert_eq!(
        why("GET", Host::Api, "/repos/o/r/%68ooks"),
        Some(Why::Credentials)
    );
}

#[test]
fn a_name_that_happens_to_spell_a_credential_is_still_read() {
    passes("GET", Host::Api, "/repos/o/r/contents/keys/readme.md");
    passes("GET", Host::Api, "/repos/o/r/labels/secrets");
    passes("GET", Host::Api, "/repos/o/r/branches/hooks");
    passes("POST", Host::Api, "/repos/o/r/issues/4/labels/secrets");
}

#[test]
fn a_repository_deleted_or_changed_is_refused() {
    assert_eq!(
        why("DELETE", Host::Api, "/repos/o/r"),
        Some(Why::RepoSettings)
    );
    assert_eq!(
        why("PATCH", Host::Api, "/repos/o/r"),
        Some(Why::RepoSettings)
    );
    assert_eq!(
        why("PATCH", Host::Api, "/repositories/123"),
        Some(Why::RepoSettings)
    );
    passes("GET", Host::Api, "/repos/o/r");
}

#[test]
fn protection_and_rulesets_are_read_but_never_written() {
    passes("GET", Host::Api, "/repos/o/r/branches/main/protection");
    passes("GET", Host::Api, "/repos/o/r/rulesets");
    passes("GET", Host::Api, "/repos/o/r/rules/branches/main");
    for (method, target) in [
        ("PUT", "/repos/o/r/branches/main/protection"),
        (
            "DELETE",
            "/repos/o/r/branches/main/protection/enforce_admins",
        ),
        (
            "POST",
            "/repos/o/r/branches/feature/x/protection/required_signatures",
        ),
        ("POST", "/repos/o/r/rulesets"),
        ("PUT", "/repos/o/r/rulesets/3"),
        ("POST", "/orgs/o/rulesets"),
    ] {
        assert_eq!(
            why(method, Host::Api, target),
            Some(Why::Protection),
            "{method} {target}"
        );
    }
}

#[test]
fn every_other_write_is_refused() {
    for (method, target) in [
        ("PUT", "/repos/o/r/contents/README.md"),
        ("POST", "/repos/o/r/git/refs"),
        ("PATCH", "/repos/o/r/git/refs/heads/main"),
        ("POST", "/repos/o/r/forks"),
        ("POST", "/user/repos"),
        ("PATCH", "/user"),
        ("POST", "/repos/o/r/releases"),
        ("POST", "/repos/o/r/actions/workflows/ci.yml/dispatches"),
        ("DELETE", "/repos/o/r/git/refs/heads/main"),
        ("PUT", "/repos/o/r/collaborators/someone"),
    ] {
        assert_eq!(
            why(method, Host::Api, target),
            Some(Why::NotAllowed),
            "{method} {target}"
        );
    }
    assert_eq!(
        why("POST", Host::Uploads, "/repos/o/r/releases/1/assets"),
        Some(Why::NotAllowed)
    );
    assert_eq!(why("TRACE", Host::Api, "/user"), Some(Why::NotAllowed));
}

#[test]
fn a_path_github_might_read_as_another_is_refused() {
    for target in [
        "/repos/o/r/issues/../../hooks",
        "/repos/o/r/./hooks",
        "/repos//o/r/hooks",
        "/repos/o/r/issues%2F..%2Fhooks",
        "/repos/o/r/%2e%2e/hooks",
        "/repos/o/r/%zz",
        "relative",
    ] {
        assert_eq!(
            why("GET", Host::Api, target),
            Some(Why::Unreadable),
            "{target}"
        );
    }
}

#[test]
fn a_method_parameter_is_refused() {
    assert_eq!(
        why(
            "POST",
            Host::Api,
            "/repos/o/r/issues/4/comments?_method=DELETE"
        ),
        Some(Why::MethodOverride)
    );
}

#[test]
fn graphql_is_judged_by_its_document() {
    let body = br#"{"query": "mutation { deleteRepository(input: {}) { clientMutationId } }"}"#;
    let call = Call {
        method: "POST",
        host: Host::Api,
        target: "/graphql",
        body,
    };
    assert_eq!(judge(&call, &[]).expect_err("refused").why, Why::Mutation);
    let get = "/graphql?query=mutation%20%7B%20addSubIssue(input%3A%20%7B%7D)\
        %20%7B%20clientMutationId%20%7D%20%7D";
    assert_eq!(
        judge(
            &Call {
                method: "GET",
                host: Host::Api,
                target: get,
                body: b""
            },
            &["addSubIssue"]
        )
        .expect_err("refused")
        .why,
        Why::GetMutation
    );
    assert_eq!(why("PUT", Host::Api, "/graphql"), Some(Why::NotAllowed));
}

#[test]
fn git_fetch_passes_and_a_push_is_refused_with_its_route() {
    passes("POST", Host::Web, "/o/r.git/git-upload-pack");
    passes("POST", Host::Web, "/o/r/git-upload-pack");
    let push = judged("POST", Host::Web, "/o/r.git/git-receive-pack").expect_err("refused");
    assert_eq!(push.why, Why::ReceivePack);
    assert!(
        push.detail.contains("work tool"),
        "a push names the work tool"
    );
    assert_eq!(
        why(
            "GET",
            Host::Web,
            "/o/r.git/info/refs?service=git-receive-pack"
        ),
        Some(Why::ReceivePack)
    );
    assert_eq!(why("POST", Host::Web, "/session"), Some(Why::NotAllowed));
    assert_eq!(
        why("POST", Host::Web, "/o/r/issues/new"),
        Some(Why::NotAllowed)
    );
}

#[test]
fn an_lfs_download_passes_and_an_upload_is_a_push() {
    let batch = |body: &[u8]| {
        judge(
            &Call {
                method: "POST",
                host: Host::Web,
                target: "/o/r.git/info/lfs/objects/batch",
                body,
            },
            &[],
        )
    };
    assert!(
        batch(br#"{"operation": "download", "objects": []}"#).is_ok(),
        "an LFS download passes"
    );
    assert_eq!(
        batch(br#"{"operation": "upload", "objects": []}"#)
            .expect_err("refused")
            .why,
        Why::ReceivePack
    );
    assert_eq!(batch(b"{}").expect_err("refused").why, Why::Unreadable);
}
