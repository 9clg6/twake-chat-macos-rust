//! `cargo xtask check-deps`: ARCHITECTURE.md section 3 (table of internal edges,
//! P2 exclusive external systems, hard rules 1 to 4) as a pure function over an
//! in-memory crate graph. No `cargo metadata` here, except the single
//! `#[ignore]`d test at the end.
//! Public items used are listed in `docs/dev/api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::path::Path;

use xtask::check_deps::{CrateInfo, RuleId, Rules, Violation, check, load_graph};

// ------------------------------------------------------------ the table of section 3

/// Allowed internal edges, copied from ARCHITECTURE.md section 3 ("May depend on").
/// `twake-desktop` ("all") is expanded; `twake-testing` is dev-dependency only
/// and so appears in no normal list.
const ALLOWED: &[(&str, &[&str])] = &[
    ("twake-domain", &[]),
    ("twake-app", &["twake-domain"]),
    ("twake-matrix", &["twake-app", "twake-domain"]),
    ("twake-services", &["twake-app", "twake-domain"]),
    ("twake-media", &["twake-app", "twake-domain"]),
    ("twake-store", &["twake-app", "twake-domain"]),
    ("twake-platform", &["twake-app", "twake-domain"]),
    ("twake-calls", &["twake-app", "twake-domain"]),
    ("twake-i18n", &["twake-domain"]),
    ("twake-tokens", &[]),
    ("twake-icons", &[]),
    ("twake-gpui", &[]),
    (
        "twake-ui-kit",
        &["twake-gpui", "twake-tokens", "twake-icons", "twake-domain"],
    ),
    (
        "twake-ui",
        &[
            "twake-ui-kit",
            "twake-gpui",
            "twake-app",
            "twake-domain",
            "twake-i18n",
        ],
    ),
    ("twake-testing", &["twake-app", "twake-domain"]),
    (
        "twake-desktop",
        &[
            "twake-matrix",
            "twake-services",
            "twake-media",
            "twake-store",
            "twake-platform",
            "twake-calls",
            "twake-app",
            "twake-domain",
            "twake-i18n",
            "twake-tokens",
            "twake-icons",
            "twake-gpui",
            "twake-ui-kit",
            "twake-ui",
        ],
    ),
    ("xtask", &[]),
];

const ADAPTERS: &[&str] = &[
    "twake-matrix",
    "twake-services",
    "twake-media",
    "twake-store",
    "twake-platform",
    "twake-calls",
];

/// Exclusive external systems (P2): (dependency, crate that owns it).
const EXCLUSIVE: &[(&str, &str)] = &[
    ("matrix-sdk", "twake-matrix"),
    ("matrix-sdk-ui", "twake-matrix"),
    ("matrix-sdk-base", "twake-matrix"),
    ("gpui", "twake-gpui"),
    ("gpui_tokio", "twake-gpui"),
    ("gpui_macros", "twake-gpui"),
    ("livekit", "twake-calls"),
    ("keyring", "twake-store"),
    ("keyring-core", "twake-store"),
    ("reqwest", "twake-services"),
];

/// Externals every crate of the table may use freely (P8, free under section 3).
const FREE: &[&str] = &[
    "serde",
    "thiserror",
    "futures",
    "async-trait",
    "imbl",
    "zeroize",
    "fluent",
];

fn names() -> Vec<&'static str> {
    ALLOWED.iter().map(|(n, _)| *n).collect()
}

fn natural_externals(name: &str) -> Vec<&'static str> {
    match name {
        "twake-matrix" => vec!["matrix-sdk", "matrix-sdk-ui", "tokio", "serde"],
        "twake-gpui" => vec!["gpui", "tokio"],
        "twake-services" => vec!["reqwest", "serde"],
        "twake-store" => vec!["keyring-core", "zeroize"],
        "twake-calls" => vec!["livekit"],
        "twake-media" => vec!["image"],
        "xtask" => vec!["cargo_metadata", "anyhow"],
        "twake-domain" | "twake-app" => vec!["serde", "thiserror", "imbl", "futures"],
        // Tokens and icons are dependency-free: nothing.
        "twake-tokens" | "twake-icons" => vec![],
        _ => vec!["serde"],
    }
}

/// The valid graph of section 3: each crate with exactly its allowed edges
/// plus the externals it naturally uses.
fn valid_graph() -> Vec<CrateInfo> {
    ALLOWED
        .iter()
        .map(|(name, deps)| {
            let mut c = CrateInfo::new(name);
            for d in *deps {
                c = c.dep(d);
            }
            for e in natural_externals(name) {
                c = c.external(e);
            }
            // The fakes are a dev-dependency of everyone who needs them.
            if *name != "twake-testing" && *name != "xtask" {
                c = c.dev_dep("twake-testing");
            }
            c
        })
        .collect()
}

fn rules() -> Rules {
    Rules::twake()
}

fn v(krate: &str, dependency: &str, rule: RuleId) -> Violation {
    Violation {
        krate: krate.into(),
        dependency: dependency.into(),
        rule,
    }
}

/// The graph of section 3 with one more normal internal edge.
fn with_edge(from: &str, to: &str) -> Vec<CrateInfo> {
    valid_graph()
        .into_iter()
        .map(|c| if c.name == from { c.dep(to) } else { c })
        .collect()
}

/// The graph of section 3 with one more external dependency.
fn with_external(krate: &str, ext: &str) -> Vec<CrateInfo> {
    valid_graph()
        .into_iter()
        .map(|c| if c.name == krate { c.external(ext) } else { c })
        .collect()
}

// ------------------------------------------------------------ valid graph

#[test]
fn the_graph_of_section_3_has_no_violation() {
    assert_eq!(check(&rules(), &valid_graph()), vec![]);
}

#[test]
fn an_empty_graph_has_no_violation() {
    assert_eq!(check(&rules(), &[]), vec![]);
}

#[test]
fn the_order_of_the_crates_in_the_graph_does_not_matter() {
    let mut reversed = valid_graph();
    reversed.reverse();
    assert_eq!(check(&rules(), &reversed), vec![]);
    let mut broken = with_edge("twake-ui", "twake-matrix");
    let forward = check(&rules(), &broken);
    broken.reverse();
    assert_eq!(check(&rules(), &broken), forward);
}

// ------------------------------------------------------------ internal edges (table of section 3)

/// What the rules must say for one extra internal edge `from -> to`:
/// `None` when the table allows it, else the most specific rule broken.
fn expected_for_edge(from: &str, to: &str) -> Option<RuleId> {
    let allowed = ALLOWED
        .iter()
        .find(|(n, _)| *n == from)
        .unwrap()
        .1
        .contains(&to);
    if allowed {
        return None;
    }
    Some(match from {
        "twake-tokens" | "twake-icons" => RuleId::Leaf,
        "twake-domain" | "twake-app" if ADAPTERS.contains(&to) || to == "twake-gpui" => {
            RuleId::PureCore
        }
        "twake-ui" | "twake-ui-kit" if ADAPTERS.contains(&to) => RuleId::UiAdapter,
        _ => RuleId::InternalEdge,
    })
}

#[test]
fn every_internal_edge_is_accepted_when_the_table_lists_it_and_refused_otherwise() {
    // twake-desktop -> twake-testing as a *normal* edge is left out: the table
    // says "dev-dependency only" yet `--preview` runs on the fakes (question Q4).
    for from in names() {
        for to in names() {
            if from == to || (from == "twake-desktop" && to == "twake-testing") {
                continue;
            }
            let base = valid_graph();
            let already = base.iter().find(|c| c.name == from).unwrap().has_dep(to);
            let graph = if already { base } else { with_edge(from, to) };
            let found = check(&rules(), &graph);
            match expected_for_edge(from, to) {
                None => assert_eq!(found, vec![], "{from} -> {to} is in the table"),
                Some(rule) => assert_eq!(
                    found,
                    vec![v(from, to, rule)],
                    "{from} -> {to} is not in the table"
                ),
            }
        }
    }
}

#[test]
fn a_crate_depending_on_a_workspace_crate_the_table_does_not_know_is_refused() {
    let found = check(&rules(), &with_edge("twake-store", "twake-mystery"));
    assert_eq!(
        found,
        vec![v("twake-store", "twake-mystery", RuleId::InternalEdge)]
    );
}

#[test]
fn the_fakes_crate_is_a_dev_dependency_only() {
    // dev: allowed everywhere (valid_graph already adds it), domain included
    // although twake-testing depends on twake-domain (dev cycles are fine).
    assert_eq!(check(&rules(), &valid_graph()), vec![]);
    // normal: refused
    assert_eq!(
        check(&rules(), &with_edge("twake-ui", "twake-testing")),
        vec![v("twake-ui", "twake-testing", RuleId::InternalEdge)]
    );
}

#[test]
fn a_dev_dependency_follows_the_same_table_as_a_normal_one() {
    let graph: Vec<CrateInfo> = valid_graph()
        .into_iter()
        .map(|c| {
            if c.name == "twake-store" {
                c.dev_dep("twake-media")
            } else {
                c
            }
        })
        .collect();
    assert_eq!(
        check(&rules(), &graph),
        vec![v("twake-store", "twake-media", RuleId::InternalEdge)]
    );
}

// ------------------------------------------------------------ P2: one external system, one crate

#[test]
fn each_exclusive_system_is_accepted_in_its_owner() {
    // valid_graph() already gives every owner its system; here the point is
    // that nothing is flagged for the owner, prefixes included.
    for (dep, owner) in EXCLUSIVE {
        let graph = with_external(owner, dep);
        assert_eq!(check(&rules(), &graph), vec![], "{dep} in {owner}");
    }
}

#[test]
fn each_exclusive_system_is_refused_anywhere_else() {
    // Crates not caught first by a dedicated hard rule (domain/app: rule 1,
    // tokens/icons: rule 3) so that the exclusivity rule is the only one to speak.
    let elsewhere = [
        "twake-media",
        "twake-i18n",
        "twake-ui",
        "twake-ui-kit",
        "twake-desktop",
        "twake-platform",
    ];
    for (dep, owner) in EXCLUSIVE {
        for other in elsewhere {
            if other == *owner {
                continue;
            }
            let found = check(&rules(), &with_external(other, dep));
            assert_eq!(
                found,
                vec![v(other, dep, RuleId::ExclusiveSystem)],
                "{dep} in {other} (owner {owner})"
            );
        }
    }
}

#[test]
fn a_dev_dependency_on_an_exclusive_system_is_refused_too() {
    let graph: Vec<CrateInfo> = valid_graph()
        .into_iter()
        .map(|c| {
            if c.name == "twake-ui" {
                c.dev_external("matrix-sdk")
            } else {
                c
            }
        })
        .collect();
    assert_eq!(
        check(&rules(), &graph),
        vec![v("twake-ui", "matrix-sdk", RuleId::ExclusiveSystem)]
    );
}

#[test]
fn free_third_party_crates_are_allowed_everywhere_they_do_not_break_a_hard_rule() {
    for free in FREE {
        for krate in [
            "twake-ui",
            "twake-ui-kit",
            "twake-media",
            "twake-i18n",
            "twake-platform",
            "twake-desktop",
        ] {
            assert_eq!(
                check(&rules(), &with_external(krate, free)),
                vec![],
                "{free} in {krate}"
            );
        }
    }
}

// ------------------------------------------------------------ hard rule 1: domain and app are pure

#[test]
fn domain_and_app_must_not_depend_on_gpui_or_an_adapter_sdk() {
    for krate in ["twake-domain", "twake-app"] {
        for ext in ["gpui", "matrix-sdk", "livekit", "reqwest", "keyring-core"] {
            assert_eq!(
                check(&rules(), &with_external(krate, ext)),
                vec![v(krate, ext, RuleId::PureCore)],
                "{ext} in {krate}"
            );
        }
    }
}

#[test]
fn domain_and_app_must_not_depend_on_an_adapter_crate_or_on_the_gpui_crate() {
    for krate in ["twake-domain", "twake-app"] {
        for to in ADAPTERS.iter().chain(&["twake-gpui"]) {
            assert_eq!(
                check(&rules(), &with_edge(krate, to)),
                vec![v(krate, to, RuleId::PureCore)],
                "{krate} -> {to}"
            );
        }
    }
}

// ------------------------------------------------------------ hard rule 2: only the desktop imports adapters

#[test]
fn ui_crates_must_not_depend_on_an_adapter_crate() {
    for krate in ["twake-ui", "twake-ui-kit"] {
        for to in ADAPTERS {
            assert_eq!(
                check(&rules(), &with_edge(krate, to)),
                vec![v(krate, to, RuleId::UiAdapter)],
                "{krate} -> {to}"
            );
        }
    }
}

#[test]
fn the_desktop_composition_root_may_depend_on_every_adapter() {
    // valid_graph() gives twake-desktop all six adapters: no violation.
    let desktop = valid_graph()
        .into_iter()
        .find(|c| c.name == "twake-desktop")
        .unwrap();
    for a in ADAPTERS {
        assert!(desktop.has_dep(a));
    }
    assert_eq!(check(&rules(), &valid_graph()), vec![]);
}

// ------------------------------------------------------------ hard rule 3: tokens and icons are leaves

#[test]
fn tokens_and_icons_depend_on_nothing_internal() {
    for krate in ["twake-tokens", "twake-icons"] {
        for to in ["twake-domain", "twake-gpui", "twake-tokens", "twake-icons"] {
            if krate == to {
                continue;
            }
            assert_eq!(
                check(&rules(), &with_edge(krate, to)),
                vec![v(krate, to, RuleId::Leaf)],
                "{krate} -> {to}"
            );
        }
    }
}

#[test]
fn tokens_and_icons_pull_no_external_crate_in_particular_not_gpui() {
    assert_eq!(
        check(&rules(), &with_external("twake-icons", "image")),
        vec![v("twake-icons", "image", RuleId::Leaf)]
    );
    assert_eq!(
        check(&rules(), &with_external("twake-tokens", "gpui")),
        vec![v("twake-tokens", "gpui", RuleId::Leaf)]
    );
}

// ------------------------------------------------------------ hard rule 4: no cycle

#[test]
fn a_cycle_between_two_crates_is_reported_once() {
    let rules = Rules::from_edges(&[("a", "b"), ("b", "a")]);
    let graph = [CrateInfo::new("a").dep("b"), CrateInfo::new("b").dep("a")];
    assert_eq!(check(&rules, &graph), vec![v("a", "b", RuleId::Cycle)]);
}

#[test]
fn a_cycle_of_three_crates_is_reported_once_from_its_first_crate_by_name() {
    let rules = Rules::from_edges(&[("a", "b"), ("b", "c"), ("c", "a")]);
    let graph = [
        CrateInfo::new("c").dep("a"),
        CrateInfo::new("a").dep("b"),
        CrateInfo::new("b").dep("c"),
    ];
    assert_eq!(check(&rules, &graph), vec![v("a", "b", RuleId::Cycle)]);
}

#[test]
fn a_diamond_is_not_a_cycle() {
    let rules = Rules::from_edges(&[("a", "b"), ("a", "c"), ("b", "d"), ("c", "d")]);
    let graph = [
        CrateInfo::new("a").dep("b").dep("c"),
        CrateInfo::new("b").dep("d"),
        CrateInfo::new("c").dep("d"),
        CrateInfo::new("d"),
    ];
    assert_eq!(check(&rules, &graph), vec![]);
}

#[test]
fn a_crate_depending_on_itself_is_a_cycle() {
    let rules = Rules::from_edges(&[("a", "a")]);
    assert_eq!(
        check(&rules, &[CrateInfo::new("a").dep("a")]),
        vec![v("a", "a", RuleId::Cycle)]
    );
}

#[test]
fn a_back_edge_that_the_table_forbids_is_reported_once_as_an_edge_not_again_as_a_cycle() {
    // domain -> app closes a loop with app -> domain, but the fault is that one
    // edge: edges already reported are left out of cycle detection, so one
    // fault gives one report (the same holds for domain -> twake-matrix, etc.).
    assert_eq!(
        check(&rules(), &with_edge("twake-domain", "twake-app")),
        vec![v("twake-domain", "twake-app", RuleId::InternalEdge)]
    );
}

#[test]
fn a_cycle_made_of_allowed_edges_is_still_reported_when_the_table_allows_it() {
    // Rule 4 is independent of the table: here the table itself is wrong.
    let rules = Rules::from_edges(&[("a", "b"), ("b", "c"), ("c", "b")]);
    let graph = [
        CrateInfo::new("a").dep("b"),
        CrateInfo::new("b").dep("c"),
        CrateInfo::new("c").dep("b"),
    ];
    assert_eq!(check(&rules, &graph), vec![v("b", "c", RuleId::Cycle)]);
}

// ------------------------------------------------------------ unknown crate

#[test]
fn a_workspace_crate_the_table_does_not_know_is_reported() {
    let mut graph = valid_graph();
    graph.push(CrateInfo::new("twake-mystery").external("serde"));
    assert_eq!(
        check(&rules(), &graph),
        vec![v("twake-mystery", "", RuleId::UnknownCrate)]
    );
}

// ------------------------------------------------------------ several violations together

#[test]
fn two_independent_violations_are_both_reported() {
    let graph: Vec<CrateInfo> = valid_graph()
        .into_iter()
        .map(|c| match c.name.as_str() {
            "twake-ui" => c.dep("twake-matrix"),
            "twake-media" => c.external("reqwest"),
            _ => c,
        })
        .collect();
    let found = check(&rules(), &graph);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found.contains(&v("twake-ui", "twake-matrix", RuleId::UiAdapter)));
    assert!(found.contains(&v("twake-media", "reqwest", RuleId::ExclusiveSystem)));
}

#[test]
fn several_violations_in_one_crate_are_each_reported() {
    let graph: Vec<CrateInfo> = valid_graph()
        .into_iter()
        .map(|c| {
            if c.name == "twake-domain" {
                c.dep("twake-matrix")
                    .external("gpui")
                    .external("serde_json")
            } else {
                c
            }
        })
        .collect();
    let found = check(&rules(), &graph);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found.contains(&v("twake-domain", "twake-matrix", RuleId::PureCore)));
    assert!(found.contains(&v("twake-domain", "gpui", RuleId::PureCore)));
}

#[test]
fn the_report_is_in_a_stable_order_by_crate_then_dependency() {
    let graph: Vec<CrateInfo> = valid_graph()
        .into_iter()
        .map(|c| match c.name.as_str() {
            "twake-ui" => c.dep("twake-services").dep("twake-matrix"),
            "twake-icons" => c.external("image"),
            _ => c,
        })
        .collect();
    assert_eq!(
        check(&rules(), &graph),
        vec![
            v("twake-icons", "image", RuleId::Leaf),
            v("twake-ui", "twake-matrix", RuleId::UiAdapter),
            v("twake-ui", "twake-services", RuleId::UiAdapter),
        ]
    );
}

// ------------------------------------------------------------ what CI prints

#[test]
fn rule_ids_are_stable_strings() {
    let ids: Vec<&str> = [
        RuleId::InternalEdge,
        RuleId::ExclusiveSystem,
        RuleId::PureCore,
        RuleId::UiAdapter,
        RuleId::Leaf,
        RuleId::Cycle,
        RuleId::UnknownCrate,
    ]
    .iter()
    .map(|r| r.as_str())
    .collect();
    assert_eq!(
        ids,
        [
            "edge",
            "exclusive",
            "pure-core",
            "ui-adapter",
            "leaf",
            "cycle",
            "unknown-crate"
        ]
    );
}

#[test]
fn a_violation_message_names_the_crate_the_dependency_and_the_rule() {
    let text = v("twake-ui", "twake-matrix", RuleId::UiAdapter).to_string();
    assert!(text.contains("twake-ui"), "{text}");
    assert!(text.contains("twake-matrix"), "{text}");
    assert!(text.contains("ui-adapter"), "{text}");
}

// ------------------------------------------------------------ the real workspace

/// Needs `cargo metadata` at run time (spawns cargo), so it is `#[ignore]`d in
/// the unit run; CI runs it with `cargo test -p xtask -- --ignored`, and it is
/// what `cargo xtask check-deps` does on the real workspace.
#[test]
#[ignore = "runs `cargo metadata` on the real workspace"]
fn the_real_workspace_has_no_violation() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml");
    let graph = load_graph(&manifest).unwrap();
    // Not vacuous: the whole workspace was read.
    for expected in names() {
        assert!(
            graph.iter().any(|c| c.name == expected),
            "{expected} missing from the loaded graph"
        );
    }
    assert_eq!(check(&rules(), &graph), vec![]);
}
