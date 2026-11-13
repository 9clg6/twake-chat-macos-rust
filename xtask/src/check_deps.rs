//! `cargo xtask check-deps`: ARCHITECTURE.md section 3 (table of internal edges,
//! P2 exclusive external systems, hard rules 1 to 4) as a pure function over a crate graph.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

use cargo_metadata::{DependencyKind, MetadataCommand};

/// One workspace crate as read from `cargo metadata`.
#[derive(Debug, Clone)]
pub struct CrateInfo {
    /// The crate name.
    pub name: String,
    deps: BTreeSet<String>,
    dev_deps: BTreeSet<String>,
    externals: BTreeSet<String>,
    dev_externals: BTreeSet<String>,
}

impl CrateInfo {
    /// A crate with no dependency.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            deps: BTreeSet::new(),
            dev_deps: BTreeSet::new(),
            externals: BTreeSet::new(),
            dev_externals: BTreeSet::new(),
        }
    }

    fn with(mut self, set: fn(&mut Self) -> &mut BTreeSet<String>, name: &str) -> Self {
        set(&mut self).insert(name.to_owned());
        self
    }

    /// Adds a normal or build dependency on a workspace crate.
    pub fn dep(self, internal: &str) -> Self {
        self.with(|c| &mut c.deps, internal)
    }

    /// Adds a dev-dependency on a workspace crate.
    pub fn dev_dep(self, internal: &str) -> Self {
        self.with(|c| &mut c.dev_deps, internal)
    }

    /// Adds a normal or build dependency on a crate outside the workspace.
    pub fn external(self, name: &str) -> Self {
        self.with(|c| &mut c.externals, name)
    }

    /// Adds a dev-dependency on a crate outside the workspace.
    pub fn dev_external(self, name: &str) -> Self {
        self.with(|c| &mut c.dev_externals, name)
    }

    /// Does this crate have a normal dependency on the workspace crate `internal`?
    pub fn has_dep(&self, internal: &str) -> bool {
        self.deps.contains(internal)
    }
}

/// The rule a dependency breaks, most specific one is reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuleId {
    /// An internal edge absent from the table.
    InternalEdge,
    /// An external system used outside its owner crate (P2).
    ExclusiveSystem,
    /// `twake-domain` or `twake-app` depends on an adapter, GPUI or Tokio (rule 1).
    PureCore,
    /// `twake-ui*` depends on an adapter (rule 2).
    UiAdapter,
    /// `twake-tokens` or `twake-icons` has a dependency (rule 3).
    Leaf,
    /// A dependency cycle (rule 4).
    Cycle,
    /// A workspace crate the table does not list.
    UnknownCrate,
}

impl RuleId {
    /// The stable name printed in reports.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InternalEdge => "edge",
            Self::ExclusiveSystem => "exclusive",
            Self::PureCore => "pure-core",
            Self::UiAdapter => "ui-adapter",
            Self::Leaf => "leaf",
            Self::Cycle => "cycle",
            Self::UnknownCrate => "unknown-crate",
        }
    }
}

/// One offending dependency.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Violation {
    /// The crate that depends.
    pub krate: String,
    /// What it depends on (empty for an unknown crate).
    pub dependency: String,
    /// The rule broken.
    pub rule: RuleId,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} ({})",
            self.krate,
            self.dependency,
            self.rule.as_str()
        )
    }
}

/// Allowed internal edges ("May depend on" of ARCHITECTURE.md section 3).
const TABLE: &[(&str, &[&str])] = &[
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
const LEAVES: &[&str] = &["twake-tokens", "twake-icons"];
const PURE_CORE: &[&str] = &["twake-domain", "twake-app"];
const UI: &[&str] = &["twake-ui", "twake-ui-kit"];
/// Workspace crates `twake-domain` and `twake-app` must not reach, besides the adapters.
const PURE_CORE_INTERNAL_BANS: &[&str] = &["twake-gpui"];
/// Externals a leaf may carry (rule 3 allows `serde` behind an optional feature).
const LEAF_ALLOWED: &[&str] = &["serde"];
/// Externals `twake-domain` and `twake-app` must not carry, besides the exclusive systems.
const PURE_CORE_EXTERNAL_BANS: &[&str] = &["tokio"];
/// P2: name prefix of an external system, and the only crate that may use it.
const EXCLUSIVE: &[(&str, &str)] = &[
    ("matrix-sdk", "twake-matrix"),
    ("gpui", "twake-gpui"),
    ("livekit", "twake-calls"),
    ("keyring", "twake-store"),
    ("reqwest", "twake-services"),
];
/// Any crate but this one may depend on it in dev mode only.
const FAKES: &str = "twake-testing";

/// The rules a graph is checked against.
pub struct Rules {
    allowed: BTreeMap<String, BTreeSet<String>>,
    hard: bool,
}

impl Rules {
    /// The real table of ARCHITECTURE.md section 3, hard rules and exclusive systems included.
    pub fn twake() -> Self {
        let mut rules = Self::from_edges(
            &TABLE
                .iter()
                .flat_map(|(from, tos)| tos.iter().map(move |to| (*from, *to)))
                .collect::<Vec<_>>(),
        );
        for (name, _) in TABLE {
            rules.allowed.entry((*name).to_owned()).or_default();
        }
        rules.hard = true;
        rules
    }

    /// Only an allowed-edge table: the crates it names are known; no hard rule 1 to 3, no exclusive system.
    pub fn from_edges(edges: &[(&str, &str)]) -> Self {
        let mut allowed: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (from, to) in edges {
            allowed
                .entry((*from).to_owned())
                .or_default()
                .insert((*to).to_owned());
            allowed.entry((*to).to_owned()).or_default();
        }
        Self {
            allowed,
            hard: false,
        }
    }

    fn is_known(&self, name: &str) -> bool {
        self.allowed.contains_key(name)
    }

    fn is_hard_rule_crate(&self, group: &[&str], name: &str) -> bool {
        self.hard && group.contains(&name)
    }

    /// The most specific rule broken by `krate` depending on the workspace crate `dep`.
    fn internal(&self, krate: &str, dep: &str, dev: bool) -> Option<RuleId> {
        if dev && dep == FAKES && krate != FAKES {
            return None;
        }
        self.hard_internal(krate, dep, dev).or_else(|| {
            let listed = self.allowed.get(krate).is_some_and(|tos| tos.contains(dep));
            (!listed).then_some(RuleId::InternalEdge)
        })
    }

    /// Hard rules 1 to 3 on an internal edge.
    fn hard_internal(&self, krate: &str, dep: &str, dev: bool) -> Option<RuleId> {
        let adapter = ADAPTERS.contains(&dep);
        if !dev && self.is_hard_rule_crate(LEAVES, krate) {
            Some(RuleId::Leaf)
        } else if self.is_hard_rule_crate(PURE_CORE, krate)
            && (adapter || PURE_CORE_INTERNAL_BANS.contains(&dep))
        {
            Some(RuleId::PureCore)
        } else if self.is_hard_rule_crate(UI, krate) && adapter {
            Some(RuleId::UiAdapter)
        } else {
            None
        }
    }

    /// The most specific rule broken by `krate` depending on the external crate `dep`.
    fn external(&self, krate: &str, dep: &str, dev: bool) -> Option<RuleId> {
        if !self.hard {
            return None;
        }
        if !dev && LEAVES.contains(&krate) {
            return (!LEAF_ALLOWED.contains(&dep)).then_some(RuleId::Leaf);
        }
        let owner = EXCLUSIVE
            .iter()
            .find(|(prefix, _)| dep.starts_with(prefix))
            .map(|(_, o)| *o)
            // The republished gpui crates that do not start with `gpui` (`scheduler-gpui-unofficial`...).
            .or_else(|| dep.ends_with("-gpui-unofficial").then_some("twake-gpui"));
        if PURE_CORE.contains(&krate)
            && (owner.is_some() || PURE_CORE_EXTERNAL_BANS.iter().any(|b| dep.starts_with(b)))
        {
            return Some(RuleId::PureCore);
        }
        owner
            .filter(|o| *o != krate)
            .map(|_| RuleId::ExclusiveSystem)
    }
}

/// Checks a graph; the report is sorted and does not depend on the order of `crates`.
///
/// One report per offending dependency. Cycles are searched among the normal edges that
/// were not already reported, one report per cycle.
pub fn check(rules: &Rules, crates: &[CrateInfo]) -> Vec<Violation> {
    let mut found = BTreeSet::new();
    let violation = |krate: &str, dependency: &str, rule| Violation {
        krate: krate.to_owned(),
        dependency: dependency.to_owned(),
        rule,
    };
    for c in crates {
        if !rules.is_known(&c.name) {
            found.insert(violation(&c.name, "", RuleId::UnknownCrate));
        }
        let internal = c
            .deps
            .iter()
            .map(|d| (d, false))
            .chain(c.dev_deps.iter().map(|d| (d, true)));
        for (dep, dev) in internal {
            if let Some(rule) = rules.internal(&c.name, dep, dev) {
                found.insert(violation(&c.name, dep, rule));
            }
        }
        let external = c
            .externals
            .iter()
            .map(|d| (d, false))
            .chain(c.dev_externals.iter().map(|d| (d, true)));
        for (dep, dev) in external {
            if let Some(rule) = rules.external(&c.name, dep, dev) {
                found.insert(violation(&c.name, dep, rule));
            }
        }
    }
    let reported: BTreeSet<(String, String)> = found
        .iter()
        .map(|v| (v.krate.clone(), v.dependency.clone()))
        .collect();
    found.extend(cycles(rules, crates, &reported));
    found.into_iter().collect()
}

/// One violation per cycle among the known crates, leaving out the edges in `skip`.
fn cycles(
    rules: &Rules,
    crates: &[CrateInfo],
    skip: &BTreeSet<(String, String)>,
) -> Vec<Violation> {
    let edges: BTreeMap<&str, BTreeSet<&str>> = crates
        .iter()
        .filter(|c| rules.is_known(&c.name))
        .map(|c| {
            let tos = c
                .deps
                .iter()
                .filter(|d| rules.is_known(d) && !skip.contains(&(c.name.clone(), (*d).clone())))
                .map(String::as_str)
                .collect();
            (c.name.as_str(), tos)
        })
        .collect();
    let reach = |from: &str| {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut todo: Vec<&str> = edges.get(from).into_iter().flatten().copied().collect();
        while let Some(n) = todo.pop() {
            if seen.insert(n) {
                todo.extend(edges.get(n).into_iter().flatten().copied());
            }
        }
        seen
    };
    let mut done: BTreeSet<&str> = BTreeSet::new();
    let mut out = Vec::new();
    for (&name, tos) in &edges {
        let ahead = reach(name);
        if done.contains(name) || !ahead.contains(name) {
            continue;
        }
        let ring: BTreeSet<&str> = ahead
            .iter()
            .copied()
            .filter(|m| reach(m).contains(name))
            .collect();
        done.extend(ring.iter().copied());
        // `name` is the smallest of its ring: the loop visits names in order.
        if let Some(dep) = tos.iter().find(|d| ring.contains(*d)) {
            out.push(Violation {
                krate: name.to_owned(),
                dependency: (*dep).to_owned(),
                rule: RuleId::Cycle,
            });
        }
    }
    out
}

/// Reads the workspace members and their dependencies with `cargo metadata`.
pub fn load_graph(workspace_manifest: &Path) -> Result<Vec<CrateInfo>, String> {
    let metadata = MetadataCommand::new()
        .manifest_path(workspace_manifest)
        .no_deps()
        .exec()
        .map_err(|e| e.to_string())?;
    let members: BTreeSet<String> = metadata
        .packages
        .iter()
        .map(|p| p.name.to_string())
        .collect();
    Ok(metadata
        .packages
        .iter()
        .map(|package| {
            package
                .dependencies
                .iter()
                .fold(CrateInfo::new(&package.name), |info, dep| {
                    match (
                        members.contains(&dep.name),
                        dep.kind == DependencyKind::Development,
                    ) {
                        (true, false) => info.dep(&dep.name),
                        (true, true) => info.dev_dep(&dep.name),
                        (false, false) => info.external(&dep.name),
                        (false, true) => info.dev_external(&dep.name),
                    }
                })
        })
        .collect())
}
