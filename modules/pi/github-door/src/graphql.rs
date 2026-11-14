//! GraphQL, judged by what a document would do rather than what it says
//! it's called.
//!
//! An operation's name is the client's to choose, so `mutation addComment
//! { deleteRepository(...) }` is a repository deleted. What GitHub runs is
//! each top-level field of a mutation, reached directly, under an alias,
//! or through a fragment spread or an inline fragment, so each of those is
//! found and checked against the list by its field name. A body is the
//! JSON GitHub takes, one operation at a time: a batch, a stored query
//! named by id, or a mutation over GET is refused rather than reasoned
//! about.

use std::collections::HashSet;
use std::sync::OnceLock;

use graphql_parser::query::{
    Definition, Document, FragmentDefinition, OperationDefinition, Selection, SelectionSet,
};
use serde_json::Value;

use crate::said::{Refusal, Seen, Why};

/// The mutation list, as the crate holds it.
const LIST: &str = include_str!("../mutations.json");

/// The deepest a document's brackets may nest. GitHub's own limit on
/// query depth is far below it; it's here so a document nested past any
/// real one is refused before the parser, which recurses, reads it.
const DEEPEST: usize = 64;

/// The keys a GraphQL body may hold.
const BODY_KEYS: [&str; 3] = ["query", "variables", "operationName"];

/// The keys that name a stored query rather than holding one.
const PERSISTED_KEYS: [&str; 4] = ["extensions", "id", "documentId", "doc_id"];

/// The mutations that open or merge a pull request, refused with the
/// route that does instead.
const ROUTED: [(&str, Why, &str); 4] = [
    (
        "createPullRequest",
        Why::PullCreate,
        "open a pull request with review_offer propose",
    ),
    (
        "mergePullRequest",
        Why::Merge,
        "merge with review_offer merge",
    ),
    (
        "enablePullRequestAutoMerge",
        Why::Merge,
        "merge with review_offer merge",
    ),
    ("mergeBranch", Why::Merge, "merge with review_offer merge"),
];

/// One entry of the mutation list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allowed {
    pub name: String,
    pub skill: String,
    pub call: String,
}

/// The mutation list, read once.
///
/// # Panics
///
/// When the list the crate was built with is malformed, which its tests
/// refuse to build.
pub fn allowed() -> &'static [Allowed] {
    static READ: OnceLock<Vec<Allowed>> = OnceLock::new();
    READ.get_or_init(|| read_list(LIST).expect("mutations.json is well formed"))
}

/// The entries `text` holds, each a plain field name with a skill and a
/// call, or why it can't be read.
pub fn read_list(text: &str) -> Result<Vec<Allowed>, String> {
    let value: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let entries = value
        .get("mutations")
        .and_then(Value::as_array)
        .ok_or("it holds no mutations list")?;
    let mut seen = HashSet::new();
    entries
        .iter()
        .map(|entry| {
            let field = |key: &str| {
                entry
                    .get(key)
                    .and_then(Value::as_str)
                    .filter(|text| !text.trim().is_empty())
                    .map(str::to_owned)
                    .ok_or_else(|| format!("an entry has no {key}: {entry}"))
            };
            let name = field("name")?;
            if !plain_name(&name) {
                return Err(format!("{name} isn't a plain field name"));
            }
            if !seen.insert(name.clone()) {
                return Err(format!("{name} is listed twice"));
            }
            Ok(Allowed {
                name,
                skill: field("skill")?,
                call: field("call")?,
            })
        })
        .collect()
}

/// Whether `name` is a GraphQL name and nothing more: no wildcard, no
/// pattern, nothing that could stand for kin.
fn plain_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// How a GraphQL request reached the door.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Carried {
    /// Sent by POST, as a JSON body.
    Body,
    /// Over GET, in the query string.
    Get,
}

/// The verdict on a GraphQL request whose document is `document` (a POST's
/// body, or a GET's `query` parameter), with `allowed` the mutation names
/// let through: what a line says of it, or why it's refused.
pub fn judge(document: &[u8], carried: Carried, allowed: &[&str]) -> Result<Seen, Refusal> {
    let query = match carried {
        Carried::Body => query_in(document)?,
        Carried::Get => std::str::from_utf8(document)
            .map_err(|_| Refusal::new(Why::Unparsable, "a GraphQL query that isn't UTF-8"))?
            .to_owned(),
    };
    let operations = operations(&query)?;
    let mutations: Vec<&Operation> = operations
        .iter()
        .filter(|op| op.kind == Kind::Mutation)
        .collect();
    if operations.iter().any(|op| op.kind == Kind::Subscription) {
        return Err(Refusal::new(
            Why::Subscription,
            "a GraphQL subscription, which the door doesn't carry",
        ));
    }
    if mutations.is_empty() {
        let names = operations
            .iter()
            .flat_map(|op| op.fields.iter().cloned())
            .collect();
        return Ok(Seen::Fields {
            kind: "query",
            names,
        });
    }
    let names: Vec<String> = mutations
        .iter()
        .flat_map(|op| op.fields.iter().cloned())
        .collect();
    let seen = Seen::Fields {
        kind: "mutation",
        names: names.clone(),
    };
    if carried == Carried::Get {
        return Err(Refusal::new(
            Why::GetMutation,
            "a GraphQL mutation sent over GET; a mutation is POSTed",
        )
        .seeing(seen));
    }
    if names.is_empty() {
        return Err(Refusal::new(
            Why::Unparsable,
            "a GraphQL mutation with no field",
        ));
    }
    for name in &names {
        if name == "__typename" || allowed.contains(&name.as_str()) {
            continue;
        }
        let refusal = match ROUTED.iter().find(|(routed, ..)| routed == name) {
            Some((_, why, route)) => Refusal::new(*why, format!("the mutation {name}; {route}")),
            None => Refusal::new(
                Why::Mutation,
                format!(
                    "the mutation {name}, which isn't on the door's list; a mutation is \
                     added there only with the skill that uses it"
                ),
            ),
        };
        return Err(refusal.seeing(seen));
    }
    Ok(seen)
}

/// The document a body sent by POST holds: one JSON object with a `query`
/// string, and nothing but `variables` and `operationName` beside it.
fn query_in(body: &[u8]) -> Result<String, Refusal> {
    let value: Value = serde_json::from_slice(body)
        .map_err(|_| Refusal::new(Why::Unparsable, "a GraphQL body that isn't JSON"))?;
    if value.is_array() {
        return Err(Refusal::new(
            Why::Batch,
            "several GraphQL operations in one body; send each on its own",
        ));
    }
    let Some(object) = value.as_object() else {
        return Err(Refusal::new(
            Why::Unparsable,
            "a GraphQL body that isn't an object",
        ));
    };
    if object
        .keys()
        .any(|key| PERSISTED_KEYS.contains(&key.as_str()))
    {
        return Err(Refusal::new(
            Why::Persisted,
            "a GraphQL body naming a stored query; send the query itself",
        ));
    }
    if let Some(key) = object.keys().find(|key| !BODY_KEYS.contains(&key.as_str())) {
        return Err(Refusal::new(
            Why::Unparsable,
            format!(
                "a GraphQL body holding {}, which the door doesn't read",
                crate::said::plain(key)
            ),
        ));
    }
    match object.get("query") {
        Some(Value::String(query)) => Ok(query.clone()),
        _ => Err(Refusal::new(
            Why::Persisted,
            "a GraphQL body with no query; send the query itself",
        )),
    }
}

/// What kind of operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Query,
    Mutation,
    Subscription,
}

/// An operation, and the top-level fields it would run.
#[derive(Debug)]
struct Operation {
    kind: Kind,
    fields: Vec<String>,
}

/// Every operation in `query`, each with its top-level fields found
/// through fragments, or why the document can't be read.
fn operations(query: &str) -> Result<Vec<Operation>, Refusal> {
    if too_deep(query) {
        return Err(Refusal::new(
            Why::Unparsable,
            format!("a GraphQL document nested deeper than {DEEPEST}"),
        ));
    }
    let document: Document<'_, String> = graphql_parser::parse_query(query)
        .map_err(|_| Refusal::new(Why::Unparsable, "a GraphQL document that doesn't parse"))?;
    let fragments: Vec<&FragmentDefinition<'_, String>> = document
        .definitions
        .iter()
        .filter_map(|definition| match definition {
            Definition::Fragment(fragment) => Some(fragment),
            Definition::Operation(_) => None,
        })
        .collect();
    let mut found = Vec::new();
    for definition in &document.definitions {
        let Definition::Operation(operation) = definition else {
            continue;
        };
        let (kind, set) = match operation {
            OperationDefinition::SelectionSet(set) => (Kind::Query, set),
            OperationDefinition::Query(query) => (Kind::Query, &query.selection_set),
            OperationDefinition::Mutation(mutation) => (Kind::Mutation, &mutation.selection_set),
            OperationDefinition::Subscription(subscription) => {
                (Kind::Subscription, &subscription.selection_set)
            }
        };
        let mut fields = Vec::new();
        top_fields(set, &fragments, &mut Vec::new(), &mut fields)?;
        found.push(Operation { kind, fields });
    }
    if found.is_empty() {
        return Err(Refusal::new(
            Why::Unparsable,
            "a GraphQL document with no operation",
        ));
    }
    Ok(found)
}

/// The fields `set` selects at its own level, fragments followed, into
/// `fields`. `following` holds the fragments on the way here, so a spread
/// of one already being followed is refused rather than looped through.
fn top_fields<'a>(
    set: &'a SelectionSet<'a, String>,
    fragments: &[&'a FragmentDefinition<'a, String>],
    following: &mut Vec<&'a str>,
    fields: &mut Vec<String>,
) -> Result<(), Refusal> {
    for selection in &set.items {
        match selection {
            Selection::Field(field) => fields.push(field.name.clone()),
            Selection::InlineFragment(inline) => {
                top_fields(&inline.selection_set, fragments, following, fields)?;
            }
            Selection::FragmentSpread(spread) => {
                let name = spread.fragment_name.as_str();
                if following.contains(&name) {
                    return Err(Refusal::new(
                        Why::Unparsable,
                        "a GraphQL fragment that spreads itself",
                    ));
                }
                let Some(fragment) = fragments.iter().find(|fragment| fragment.name == name) else {
                    return Err(Refusal::new(
                        Why::Unparsable,
                        "a GraphQL spread of a fragment the document doesn't define",
                    ));
                };
                following.push(name);
                top_fields(&fragment.selection_set, fragments, following, fields)?;
                following.pop();
            }
        }
    }
    Ok(())
}

/// Whether `query`'s brackets nest deeper than [`DEEPEST`], strings and
/// comments passed over, since the parser recurses on each level.
fn too_deep(query: &str) -> bool {
    let bytes = query.as_bytes();
    let mut depth = 0usize;
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'#' => {
                while at < bytes.len() && bytes[at] != b'\n' {
                    at += 1;
                }
            }
            b'"' if bytes[at..].starts_with(b"\"\"\"") => {
                at += 3;
                while at < bytes.len() && !bytes[at..].starts_with(b"\"\"\"") {
                    at += if bytes[at..].starts_with(b"\\\"\"\"") {
                        4
                    } else {
                        1
                    };
                }
                at += 2;
            }
            b'"' => {
                at += 1;
                while at < bytes.len() && bytes[at] != b'"' && bytes[at] != b'\n' {
                    at += if bytes[at] == b'\\' { 2 } else { 1 };
                }
            }
            b'{' | b'(' | b'[' => {
                depth += 1;
                if depth > DEEPEST {
                    return true;
                }
            }
            b'}' | b')' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        at += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALLOWED: [&str; 2] = ["addSubIssue", "updateProjectV2ItemPosition"];

    fn body(query: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({ "query": query })).expect("a body")
    }

    fn posted(query: &str) -> Result<Seen, Refusal> {
        judge(&body(query), Carried::Body, &ALLOWED)
    }

    fn why(result: Result<Seen, Refusal>) -> Why {
        result.expect_err("refused").why
    }

    #[test]
    fn a_query_passes_and_says_its_fields() {
        assert_eq!(
            posted("query { viewer { login } rateLimit { remaining } }"),
            Ok(Seen::Fields {
                kind: "query",
                names: vec!["viewer".to_owned(), "rateLimit".to_owned()]
            })
        );
        assert!(posted("{ viewer { login } }").is_ok(), "a query passes");
    }

    #[test]
    fn a_listed_mutation_passes() {
        assert_eq!(
            posted(
                "mutation { addSubIssue(input: {issueId: \"I\", subIssueId: \"J\"}) \
                 { issue { id } } }"
            ),
            Ok(Seen::Fields {
                kind: "mutation",
                names: vec!["addSubIssue".to_owned()]
            })
        );
    }

    #[test]
    fn a_mutation_named_for_another_is_judged_by_its_field() {
        assert_eq!(
            why(posted(
                "mutation addSubIssue { deleteRepository(input: {repositoryId: \"R\"}) \
                 { clientMutationId } }"
            )),
            Why::Mutation
        );
    }

    #[test]
    fn an_alias_is_judged_by_the_field_behind_it() {
        assert_eq!(
            why(posted(
                "mutation { addSubIssue: deleteRepository(input: {repositoryId: \"R\"}) \
                 { clientMutationId } }"
            )),
            Why::Mutation
        );
    }

    #[test]
    fn every_field_of_a_mutation_is_judged_not_only_the_first() {
        assert_eq!(
            why(posted(
                "mutation { addSubIssue(input: {}) { clientMutationId } \
                 deleteIssue(input: {}) { clientMutationId } }"
            )),
            Why::Mutation
        );
    }

    #[test]
    fn a_field_reached_through_a_fragment_is_judged() {
        let spread = "mutation { ...Harm } \
            fragment Harm on Mutation { deleteRepository(input: {}) { clientMutationId } }";
        assert_eq!(why(posted(spread)), Why::Mutation);
        let nested = "mutation { ...A } fragment A on Mutation { ...B } \
            fragment B on Mutation { deleteIssue(input: {}) { clientMutationId } }";
        assert_eq!(why(posted(nested)), Why::Mutation);
        let inline =
            "mutation { ... on Mutation { deleteRepository(input: {}) { clientMutationId } } }";
        assert_eq!(why(posted(inline)), Why::Mutation);
        let fine = "mutation { ...Fine } \
            fragment Fine on Mutation { addSubIssue(input: {}) { clientMutationId } }";
        assert!(
            posted(fine).is_ok(),
            "a listed mutation reached through a fragment passes"
        );
    }

    #[test]
    fn a_fragment_that_loops_or_isnt_there_is_refused() {
        assert_eq!(
            why(posted("mutation { ...A } fragment A on Mutation { ...A }")),
            Why::Unparsable
        );
        assert_eq!(why(posted("mutation { ...Missing }")), Why::Unparsable);
    }

    #[test]
    fn a_query_beside_a_mutation_does_not_hide_it() {
        let both = "query Fine { viewer { login } } \
            mutation Harm { deleteRepository(input: {}) { clientMutationId } }";
        assert_eq!(why(posted(both)), Why::Mutation);
    }

    #[test]
    fn a_mutation_over_get_is_refused_and_a_query_passes() {
        assert_eq!(
            why(judge(
                b"mutation { addSubIssue(input: {}) { clientMutationId } }",
                Carried::Get,
                &ALLOWED
            )),
            Why::GetMutation
        );
        assert!(
            judge(b"{ viewer { login } }", Carried::Get, &ALLOWED).is_ok(),
            "a query over GET passes"
        );
    }

    #[test]
    fn a_batch_or_a_stored_query_is_refused() {
        let batch = br#"[{"query": "{ viewer { login } }"}, {"query": "{ viewer { id } }"}]"#;
        assert_eq!(why(judge(batch, Carried::Body, &ALLOWED)), Why::Batch);
        let stored = br#"{"extensions": {"persistedQuery": {"version": 1, "sha256Hash": "abc"}}}"#;
        assert_eq!(why(judge(stored, Carried::Body, &ALLOWED)), Why::Persisted);
        let by_id = br#"{"id": "abc", "variables": {}}"#;
        assert_eq!(why(judge(by_id, Carried::Body, &ALLOWED)), Why::Persisted);
        let beside = br#"{"query": "{ viewer { login } }", "extensions": {}}"#;
        assert_eq!(why(judge(beside, Carried::Body, &ALLOWED)), Why::Persisted);
    }

    #[test]
    fn what_wont_parse_is_refused() {
        for query in [
            "mutation {",
            "",
            "fragment A on Mutation { x }",
            "{ viewer { login } ",
        ] {
            assert_eq!(why(posted(query)), Why::Unparsable, "{query}");
        }
        assert_eq!(
            why(judge(b"not json", Carried::Body, &ALLOWED)),
            Why::Unparsable
        );
        assert_eq!(
            why(judge(br#"{"query": 1}"#, Carried::Body, &ALLOWED)),
            Why::Persisted
        );
        assert_eq!(
            why(judge(
                br#"{"query": "{ a }", "x": 1}"#,
                Carried::Body,
                &ALLOWED
            )),
            Why::Unparsable
        );
    }

    #[test]
    fn a_document_nested_past_any_real_one_is_refused_before_parsing() {
        let deep = format!("{}{}", "{a".repeat(100_000), "}".repeat(100_000));
        assert_eq!(why(posted(&deep)), Why::Unparsable);
        let strings = format!("{{ a(x: \"{}\") }}", "{".repeat(200));
        assert!(posted(&strings).is_ok(), "brackets in a string don't count");
    }

    #[test]
    fn a_subscription_is_refused() {
        assert_eq!(why(posted("subscription { x { y } }")), Why::Subscription);
    }

    #[test]
    fn opening_or_merging_a_pull_request_names_its_route() {
        let open = posted("mutation { createPullRequest(input: {}) { clientMutationId } }")
            .expect_err("refused");
        assert_eq!(open.why, Why::PullCreate);
        assert!(
            open.detail.contains("review_offer propose"),
            "{}",
            open.detail
        );
        for field in ["mergePullRequest", "enablePullRequestAutoMerge"] {
            let merge = posted(&format!(
                "mutation {{ {field}(input: {{}}) {{ clientMutationId }} }}"
            ))
            .expect_err("refused");
            assert_eq!(merge.why, Why::Merge, "{field}");
            assert!(
                merge.detail.contains("review_offer merge"),
                "{}",
                merge.detail
            );
        }
    }

    #[test]
    fn the_list_is_plain_names_each_traced_to_a_skill() {
        let list = allowed();
        assert_ne!(list, []);
        for entry in list {
            assert!(
                entry.skill.ends_with("-guide")
                    || entry.skill.ends_with("-convention")
                    || entry.skill.ends_with("-format"),
                "{entry:?}"
            );
        }
        for (routed, ..) in ROUTED {
            assert!(
                !list.iter().any(|entry| entry.name == routed),
                "{routed} is refused with a route"
            );
        }
    }

    #[test]
    fn a_list_with_a_wildcard_or_no_trace_is_refused() {
        assert!(
            read_list(r#"{"mutations": [{"name": "add*", "skill": "s", "call": "c"}]}"#).is_err(),
            "a wildcard is refused"
        );
        assert!(
            read_list(r#"{"mutations": [{"name": "addX", "skill": "", "call": "c"}]}"#).is_err(),
            "an entry with no skill is refused"
        );
        assert!(
            read_list(r#"{"mutations": [{"name": "addX", "call": "c"}]}"#).is_err(),
            "an entry naming no skill is refused"
        );
        let twice = r#"{"name": "a", "skill": "s", "call": "c"}"#;
        assert!(
            read_list(&format!(r#"{{"mutations": [{twice}, {twice}]}}"#)).is_err(),
            "a name listed twice is refused"
        );
    }
}
