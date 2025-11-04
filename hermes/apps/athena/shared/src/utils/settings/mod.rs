//! Command line and environment variable settings for the service
use cardano_blockchain_types::Network;
use log::error;
use std::sync::LazyLock;
use url::Url;

pub(crate) mod chain_follower;
pub(crate) mod str_env_var;

/// Default Github repo owner
const GITHUB_REPO_OWNER_DEFAULT: &str = "input-output-hk";

/// Default Github repo name
const GITHUB_REPO_NAME_DEFAULT: &str = "hermes";

/// Default Github issue template to use
const GITHUB_ISSUE_TEMPLATE_DEFAULT: &str = "bug_report.yml";

/// All the `EnvVars` used by the service.
struct EnvVars {
    /// The github repo owner
    github_repo_owner: &'static str,

    /// The github repo name
    github_repo_name: &'static str,

    /// The github issue template to use
    github_issue_template: &'static str,

    /// The Chain Follower configuration
    chain_follower: chain_follower::EnvVars,
}

// Lazy initialization of all env vars which are not command line parameters.
// All env vars used by the application should be listed here and all should have a
// default. The default for all NON Secret values should be suitable for Production, and
// NOT development. Secrets however should only be used with the default value in
// development

/// Handle to the mithril sync thread. One for each Network ONLY.
static ENV_VARS: LazyLock<EnvVars> = LazyLock::new(|| {
    // Support env vars in a `.env` file,  doesn't need to exist.

    // TODO: get vars from env correctly after filesystem implemented in host
    EnvVars {
        github_repo_owner: option_env!("GITHUB_REPO_OWNER").unwrap_or(GITHUB_REPO_OWNER_DEFAULT),
        github_repo_name: option_env!("GITHUB_REPO_NAME").unwrap_or(GITHUB_REPO_NAME_DEFAULT),
        github_issue_template: option_env!("GITHUB_ISSUE_TEMPLATE")
            .unwrap_or(GITHUB_ISSUE_TEMPLATE_DEFAULT),
        chain_follower: chain_follower::EnvVars::new(),
    }
});

/// Our Global Settings for this running service.
pub struct Settings();

impl Settings {
    /// Chain Follower network (The Blockchain network we are configured to use).
    /// Note: Catalyst Gateway can ONLY follow one network at a time.
    #[must_use]
    pub fn cardano_network() -> Network {
        ENV_VARS.chain_follower.chain.clone()
    }

    /// Generate a github issue url with a given title
    pub(crate) fn generate_github_issue_url(title: &str) -> Option<Url> {
        let path = format!(
            "https://github.com/{}/{}/issues/new",
            ENV_VARS.github_repo_owner, ENV_VARS.github_repo_name
        );

        match Url::parse_with_params(
            &path,
            &[
                ("template", ENV_VARS.github_issue_template),
                ("title", title),
            ],
        ) {
            Ok(url) => Some(url),
            Err(e) => {
                error!("Failed to generate github issue url {:?}", e.to_string());
                None
            },
        }
    }
}
