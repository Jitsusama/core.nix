use super::*;

const NOW: i64 = 1_795_723_200;
const FORGE: &str = "forge.example";

fn sampled(ago: i64, tls: Option<f64>) -> String {
    match tls {
        Some(tls) => serde_json::json!({
            "at": NOW - ago, "origin": FORGE, "device": "tunnel0",
            "connect_s": 0.1, "tls_s": tls, "first_s": tls + 0.1, "code": 200,
        }),
        None => serde_json::json!({
            "at": NOW - ago, "origin": FORGE, "device": "tunnel0",
            "error": "curl exited 28",
        }),
    }
    .to_string()
}

fn of(lines: &[String]) -> Measure {
    let measures = path(&lines.join("\n"), &[FORGE.to_owned()], NOW);
    assert_eq!(
        measures.len(),
        1,
        "one measure for one origin: {measures:?}"
    );
    measures[0].clone()
}

#[test]
fn the_hours_median_tls_setup_is_held_to_its_ceiling() {
    let starved = of(&[
        sampled(100, Some(6.1)),
        sampled(700, Some(9.8)),
        sampled(1300, Some(7.2)),
    ]);
    assert_eq!(starved.key, "path:tls:forge.example");
    assert_eq!(starved.now, 7.2, "the median of three");
    assert!(starved.past().is_some(), "7.2 s is past 1 s");
    assert_eq!(starved.behind, ["via tunnel0", "worst 9.80 s"]);
    let unshaped = of(&[
        sampled(100, Some(0.25)),
        sampled(700, Some(0.28)),
        sampled(1300, Some(1.0)),
    ]);
    assert_eq!(unshaped.past(), None, "a median of 0.28 s is in");
}

#[test]
fn a_failed_sample_counts_as_the_probes_whole_wait() {
    let m = of(&[
        sampled(100, None),
        sampled(700, None),
        sampled(1300, Some(0.3)),
    ]);
    assert_eq!(m.now, PROBE_WAIT, "two of three failed");
    assert_eq!(
        m.behind[2], "2 of 3 failed, as curl exited 28",
        "the failure is said: {:?}",
        m.behind
    );
}

#[test]
fn too_few_samples_in_the_hour_leave_it_unjudged() {
    let m = of(&[
        sampled(100, Some(0.2)),
        sampled(3700, Some(0.2)),
        sampled(-5, Some(0.2)),
    ]);
    assert_eq!(m.target, None, "one sample in the hour: {m:?}");
    assert!(
        m.says.contains("1 of its samples"),
        "it says how many: {}",
        m.says
    );
}

#[test]
fn an_origin_named_but_never_sampled_is_unjudged_not_gone() {
    let measures = path("", &[FORGE.to_owned(), "model.example".to_owned()], NOW);
    assert_eq!(measures.len(), 2, "one for each origin named: {measures:?}");
    assert!(
        measures.iter().all(|m| m.target.is_none()),
        "no sample is unjudged: {measures:?}"
    );
}

#[test]
fn the_path_is_said_beside_the_calls_that_took_it() {
    let m = of(&[
        sampled(100, Some(0.25)),
        sampled(700, Some(0.25)),
        sampled(1300, Some(0.3)),
    ]);
    let unread = path("", &["model.example".to_owned()], NOW);
    assert_eq!(
        said(&[m, unread[0].clone()]),
        ["TLS to forge.example via tunnel0: median 0.25 s"],
        "only a judged path is said"
    );
}
