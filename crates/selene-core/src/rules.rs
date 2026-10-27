use serde::Deserialize;

/// Mojang's rule lists decide which libraries and arguments apply on this machine.
///
/// With no rules everything applies. Otherwise nothing applies until a matching rule allows it,
/// and the last matching rule wins.
pub fn allowed(rules: &[Rule], env: &Environment) -> bool {
    if rules.is_empty() {
        return true;
    }
    rules
        .iter()
        .rfind(|rule| rule.matches(env))
        .is_some_and(|rule| rule.action == Action::Allow)
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub action: Action,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<Features>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Allow,
    Disallow,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Features {
    pub is_demo_user: Option<bool>,
    pub has_custom_resolution: Option<bool>,
    pub has_quick_plays_support: Option<bool>,
    pub is_quick_play_singleplayer: Option<bool>,
    pub is_quick_play_multiplayer: Option<bool>,
    pub is_quick_play_realms: Option<bool>,
}

/// The machine and launch options rules are checked against.
#[derive(Debug, Clone, Copy)]
pub struct Environment {
    pub os: &'static str,
    pub arch: &'static str,
    pub features: Features,
}

impl Environment {
    pub fn current(features: Features) -> Self {
        Self {
            os: match std::env::consts::OS {
                "macos" => "osx",
                other => other,
            },
            arch: match std::env::consts::ARCH {
                "x86_64" => "x86_64",
                "x86" => "x86",
                "aarch64" => "arm64",
                other => other,
            },
            features,
        }
    }
}

impl Rule {
    fn matches(&self, env: &Environment) -> bool {
        let os_matches = self.os.as_ref().is_none_or(|os| {
            os.name.as_deref().is_none_or(|name| name == env.os)
                && os.arch.as_deref().is_none_or(|arch| arch == env.arch)
        });
        let features_match = self
            .features
            .as_ref()
            .is_none_or(|features| features.satisfied_by(env.features));
        os_matches && features_match
    }
}

impl Features {
    /// Every feature the rule names must have the same value in the launch options; a feature
    /// the launch options leave unset counts as `false`.
    fn satisfied_by(self, enabled: Features) -> bool {
        let pairs = [
            (self.is_demo_user, enabled.is_demo_user),
            (self.has_custom_resolution, enabled.has_custom_resolution),
            (
                self.has_quick_plays_support,
                enabled.has_quick_plays_support,
            ),
            (
                self.is_quick_play_singleplayer,
                enabled.is_quick_play_singleplayer,
            ),
            (
                self.is_quick_play_multiplayer,
                enabled.is_quick_play_multiplayer,
            ),
            (self.is_quick_play_realms, enabled.is_quick_play_realms),
        ];
        pairs
            .into_iter()
            .all(|(wanted, actual)| wanted.is_none_or(|wanted| wanted == actual.unwrap_or(false)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(os: &'static str) -> Environment {
        Environment {
            os,
            arch: "x86_64",
            features: Features::default(),
        }
    }

    fn rules(json: &str) -> Vec<Rule> {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn no_rules_allow_everything() {
        assert!(allowed(&[], &env("windows")));
    }

    #[test]
    fn allow_only_on_one_os() {
        let rules = rules(r#"[{ "action": "allow", "os": { "name": "osx" } }]"#);

        assert!(allowed(&rules, &env("osx")));
        assert!(!allowed(&rules, &env("windows")));
    }

    #[test]
    fn last_matching_rule_wins() {
        let rules = rules(
            r#"[
                { "action": "allow" },
                { "action": "disallow", "os": { "name": "osx" } }
            ]"#,
        );

        assert!(allowed(&rules, &env("linux")));
        assert!(!allowed(&rules, &env("osx")));
    }

    #[test]
    fn features_must_be_enabled() {
        let rules =
            rules(r#"[{ "action": "allow", "features": { "has_custom_resolution": true } }]"#);
        let mut with_resolution = env("windows");
        with_resolution.features.has_custom_resolution = Some(true);

        assert!(!allowed(&rules, &env("windows")));
        assert!(allowed(&rules, &with_resolution));
    }
}
