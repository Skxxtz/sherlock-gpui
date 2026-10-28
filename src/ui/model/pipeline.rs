use std::{collections::HashMap, sync::Arc};

use crate::{
    launcher::{LauncherConfig, LauncherId},
    ui::utils::scoring::SortKey,
    utils::config::HomeType,
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum FilterPolicy {
    #[default]
    All,
    QueryOnly,
}

/// Outcomes of rules 1-4 for a whole launcher.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupGate {
    /// Rules 1-3 reject the launcher; none of its items are shown.
    Hidden,
    /// Rule 4: every item is shown regardless of the query
    Persist,
    /// Items are shown if they pass rules 5-6
    Match,
}

/// The launcher-level settings rules 1–4 and the limit need.
/// Plain data (`Send + Sync`), so it can be used off the main thread.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupRules {
    pub alias: Option<Arc<str>>,
    pub priority: u16,
    pub home: HomeType,
    /// The launcher and its limit, if it has one.
    pub limit: Option<(LauncherId, u16)>,
}

impl GroupRules {
    /// A group without alias, home restrictions or limit: shown on home and while searching
    /// in the default mode, hidden in alias modes. For data that has no launchers.
    pub fn plain() -> Self {
        Self {
            alias: None,
            priority: 1,
            home: HomeType::Home,
            limit: None,
        }
    }
}

impl From<&LauncherConfig> for GroupRules {
    fn from(cfg: &LauncherConfig) -> Self {
        Self {
            alias: cfg.alias.as_deref().map(Arc::from),
            priority: cfg.priority,
            home: cfg.home,
            limit: limit_of(cfg),
        }
    }
}

impl FilterPolicy {
    #[inline(always)]
    pub fn uses_limit(self) -> bool {
        self == Self::All
    }
}

pub struct FilterCtx<'a> {
    pub policy: FilterPolicy,
    pub mode: &'a str,
    pub query: &'a str,
}

impl<'a> FilterCtx<'a> {
    pub fn new(policy: FilterPolicy, mode: &'a str, query: &'a str) -> Self {
        Self {
            policy,
            mode,
            query,
        }
    }

    #[inline(always)]
    pub fn is_home(&self) -> bool {
        self.query.is_empty() && self.mode == "all"
    }

    pub fn gate(&self, cfg: &GroupRules) -> GroupGate {
        if self.policy == FilterPolicy::QueryOnly {
            return GroupGate::Match;
        }

        // [Rule 1]
        // Case 1: mode applies but the launcher is not assigned to that mode
        // Case 2: current mode is not the launcher's required mode
        if Some(self.mode) != cfg.alias.as_deref() && (self.mode != "all" || cfg.priority < 1) {
            return GroupGate::Hidden;
        }

        let is_home = self.is_home();
        match cfg.home {
            // [Rule 2]
            HomeType::OnlyHome if !is_home => GroupGate::Hidden,
            // [Rule 3]
            HomeType::Search if is_home => GroupGate::Hidden,
            // [Rule 4]
            HomeType::Persist => GroupGate::Persist,
            _ => GroupGate::Match,
        }
    }
}

/// Rules 4-6 for a single item.
/// `based_show` and `fuzzy` are lazy: they only run when the earlier rules dont already decide.
#[inline(always)]
pub fn admit(
    gate: GroupGate,
    based_show: impl FnOnce() -> Option<bool>,
    fuzzy: impl FnOnce() -> bool,
) -> bool {
    match gate {
        GroupGate::Hidden => false,
        GroupGate::Persist => true,
        GroupGate::Match => based_show().unwrap_or_else(fuzzy),
    }
}

/// An item that passes filtering, waiting to be sorted and limited.
pub struct Scored<I> {
    pub index: I,
    pub prio: SortKey,
    /// The owning launcher and its limit, if it has one.
    pub limit: Option<(LauncherId, u16)>,
}

/// The `(launcher, limit)` pair for `Scored::limit`.
/// Computed once per launcher.
#[inline]
pub fn limit_of(cfg: &LauncherConfig) -> Option<(LauncherId, u16)> {
    cfg.limit.map(|limit| (LauncherId::from(cfg), limit))
}

pub fn sort_and_limit<I>(mut results: Vec<Scored<I>>, policy: FilterPolicy) -> Arc<[I]> {
    results.sort_unstable_by_key(|r| r.prio);

    let use_limit = policy.uses_limit();
    let mut counts: HashMap<LauncherId, u16> = HashMap::new();

    results
        .into_iter()
        .filter_map(|r| match r.limit {
            Some((id, limit)) if use_limit => {
                let count = counts.entry(id).or_insert(0);
                *count += 1;
                (*count <= limit).then_some(r.index)
            }
            _ => Some(r.index),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::utils::Priority;

    fn cfg(alias: Option<&str>, priority: u16, home: HomeType) -> GroupRules {
        GroupRules::from(&LauncherConfig {
            alias: alias.map(str::to_owned),
            priority,
            home,
            ..Default::default()
        })
    }

    fn key(base: u16) -> SortKey {
        Priority {
            base,
            count: 0,
            tie: 0,
        }
        .sort_key("", "", 0.0)
    }

    const ALL: FilterPolicy = FilterPolicy::All;
    const HOME: FilterCtx<'static> = FilterCtx {
        policy: ALL,
        mode: "all",
        query: "",
    };
    const SEARCH: FilterCtx<'static> = FilterCtx {
        policy: ALL,
        mode: "all",
        query: "fire",
    };

    // --- Rule 1 -----------------------------------------------------------

    #[test]
    fn rule1_alias_mode_only_shows_its_launcher() {
        let ctx = FilterCtx::new(ALL, "app", "fire");
        assert_eq!(
            ctx.gate(&cfg(Some("app"), 1, HomeType::Home)),
            GroupGate::Match
        );
        assert_eq!(
            ctx.gate(&cfg(Some("web"), 1, HomeType::Home)),
            GroupGate::Hidden
        );
        assert_eq!(ctx.gate(&cfg(None, 1, HomeType::Home)), GroupGate::Hidden);
    }

    #[test]
    fn rule1_alias_mode_shows_prio_zero_launcher() {
        let ctx = FilterCtx::new(ALL, "app", "");
        assert_eq!(
            ctx.gate(&cfg(Some("app"), 0, HomeType::Home)),
            GroupGate::Match
        );
    }

    #[test]
    fn rule1_all_mode_hides_prio_zero() {
        assert_eq!(
            SEARCH.gate(&cfg(None, 0, HomeType::Home)),
            GroupGate::Hidden
        );
        assert_eq!(
            SEARCH.gate(&cfg(Some("app"), 0, HomeType::Home)),
            GroupGate::Hidden
        );
        assert_eq!(
            SEARCH.gate(&cfg(Some("app"), 1, HomeType::Home)),
            GroupGate::Match
        );
    }

    #[test]
    fn alias_mode_is_never_home() {
        assert!(!FilterCtx::new(ALL, "app", "").is_home());
        assert!(HOME.is_home());
        assert!(!SEARCH.is_home());
    }

    // --- Rules 2 & 3 --------------------------------------------------------

    #[test]
    fn rule2_only_home_hidden_while_searching() {
        let c = cfg(None, 1, HomeType::OnlyHome);
        assert_eq!(HOME.gate(&c), GroupGate::Match);
        assert_eq!(SEARCH.gate(&c), GroupGate::Hidden);
    }

    #[test]
    fn rule2_only_home_hidden_in_alias_mode() {
        let c = cfg(Some("app"), 1, HomeType::OnlyHome);
        assert_eq!(FilterCtx::new(ALL, "app", "").gate(&c), GroupGate::Hidden);
    }

    #[test]
    fn rule3_search_hidden_on_home() {
        let c = cfg(None, 1, HomeType::Search);
        assert_eq!(HOME.gate(&c), GroupGate::Hidden);
        assert_eq!(SEARCH.gate(&c), GroupGate::Match);
    }

    #[test]
    fn home_type_shows_everywhere() {
        let c = cfg(None, 1, HomeType::Home);
        assert_eq!(HOME.gate(&c), GroupGate::Match);
        assert_eq!(SEARCH.gate(&c), GroupGate::Match);
    }

    // --- Rule 4 -------------------------------------------------------------

    #[test]
    fn rule4_persist_on_home_and_search() {
        let c = cfg(None, 1, HomeType::Persist);
        assert_eq!(HOME.gate(&c), GroupGate::Persist);
        assert_eq!(SEARCH.gate(&c), GroupGate::Persist);
    }

    #[test]
    fn rule4_persist_still_respects_rule1() {
        let c = cfg(Some("web"), 1, HomeType::Persist);
        assert_eq!(FilterCtx::new(ALL, "app", "x").gate(&c), GroupGate::Hidden);
    }

    // --- QueryOnly ----------------------------------------------------------

    #[test]
    fn query_only_ignores_launcher_rules() {
        let ctx = FilterCtx::new(FilterPolicy::QueryOnly, "all", "");
        assert_eq!(ctx.gate(&cfg(None, 0, HomeType::Search)), GroupGate::Match);
        assert_eq!(ctx.gate(&cfg(None, 1, HomeType::Persist)), GroupGate::Match);
        assert_eq!(
            ctx.gate(&cfg(Some("web"), 1, HomeType::OnlyHome)),
            GroupGate::Match
        );
    }

    // --- Rules 5 & 6 --------------------------------------------------------

    #[test]
    fn admit_hidden_runs_nothing() {
        assert!(!admit(
            GroupGate::Hidden,
            || panic!("based"),
            || panic!("fuzzy")
        ));
    }

    #[test]
    fn admit_persist_runs_nothing() {
        assert!(admit(
            GroupGate::Persist,
            || panic!("based"),
            || panic!("fuzzy")
        ));
    }

    #[test]
    fn rule5_based_show_overrides_fuzzy() {
        assert!(admit(GroupGate::Match, || Some(true), || panic!("fuzzy")));
        assert!(!admit(GroupGate::Match, || Some(false), || panic!("fuzzy")));
    }

    #[test]
    fn rule6_fuzzy_when_no_based_show() {
        assert!(admit(GroupGate::Match, || None, || true));
        assert!(!admit(GroupGate::Match, || None, || false));
    }

    // --- Sort & limit -------------------------------------------------------

    fn scored(index: u32, base: u16, limit: Option<(u64, u16)>) -> Scored<u32> {
        Scored {
            index,
            prio: key(base),
            limit: limit.map(|(id, l)| (LauncherId(id), l)),
        }
    }

    #[test]
    fn sorts_by_priority() {
        let r = vec![scored(0, 3, None), scored(1, 1, None), scored(2, 2, None)];
        assert_eq!(&*sort_and_limit(r, ALL), &[1, 2, 0]);
    }

    #[test]
    fn limit_keeps_best_n_per_launcher() {
        let r = vec![
            scored(0, 4, Some((1, 2))),
            scored(1, 1, Some((1, 2))),
            scored(2, 2, Some((1, 2))),
            scored(3, 3, None),
            scored(4, 0, Some((2, 1))),
            scored(5, 5, Some((2, 1))),
        ];
        assert_eq!(&*sort_and_limit(r, ALL), &[4, 1, 2, 3]);
    }

    #[test]
    fn query_only_ignores_limit() {
        let r = vec![scored(0, 1, Some((1, 1))), scored(1, 2, Some((1, 1)))];
        assert_eq!(&*sort_and_limit(r, FilterPolicy::QueryOnly), &[0, 1]);
    }

    #[test]
    fn limit_of_uses_config() {
        let mut c = LauncherConfig::default();
        assert_eq!(limit_of(&c), None);
        c.limit = Some(3);
        assert_eq!(limit_of(&c), Some((LauncherId::from(&c), 3)));
    }

    #[test]
    fn group_rules_from_config() {
        let c = LauncherConfig {
            alias: Some("app".into()),
            priority: 2,
            home: HomeType::Persist,
            limit: Some(5),
            ..Default::default()
        };
        let g = GroupRules::from(&c);
        assert_eq!(g.alias.as_deref(), Some("app"));
        assert_eq!(g.priority, 2);
        assert_eq!(g.home, HomeType::Persist);
        assert_eq!(g.limit, Some((LauncherId::from(&c), 5)));
    }
}
