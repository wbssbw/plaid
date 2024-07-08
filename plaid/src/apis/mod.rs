pub mod general;
pub mod github;
pub mod okta;
pub mod pagerduty;
pub mod quorum;
pub mod rustica;
pub mod slack;
pub mod splunk;
pub mod web;
pub mod yubikey;

use crossbeam_channel::Sender;
use serde::Deserialize;
use tokio::runtime::Runtime;

use general::{General, GeneralConfig};
use github::{Github, GithubConfig};
use okta::{Okta, OktaConfig};
use pagerduty::{PagerDuty, PagerDutyConfig};
use quorum::{Quorum, QuorumConfig};
use slack::{Slack, SlackConfig};
use splunk::{Splunk, SplunkConfig};
use web::{Web, WebConfig};
use yubikey::{Yubikey, YubikeyConfig};

use crate::{data::DelayedMessage, executor::Message};

use self::rustica::{Rustica, RusticaConfig};

pub struct Api {
    pub runtime: Runtime,
    pub general: Option<General>,
    pub github: Option<Github>,
    pub okta: Option<Okta>,
    pub pagerduty: Option<PagerDuty>,
    pub quorum: Option<Quorum>,
    pub rustica: Option<Rustica>,
    pub slack: Option<Slack>,
    pub splunk: Option<Splunk>,
    pub yubikey: Option<Yubikey>,
    pub web: Option<Web>,
}

#[derive(Deserialize)]
pub struct Apis {
    pub general: Option<GeneralConfig>,
    pub github: Option<GithubConfig>,
    pub okta: Option<OktaConfig>,
    pub pagerduty: Option<PagerDutyConfig>,
    pub quorum: Option<QuorumConfig>,
    pub rustica: Option<RusticaConfig>,
    pub slack: Option<SlackConfig>,
    pub splunk: Option<SplunkConfig>,
    pub yubikey: Option<YubikeyConfig>,
    pub web: Option<WebConfig>,
}

#[derive(Debug)]
pub enum ApiError {
    BadRequest,
    ImpossibleError,
    ConfigurationError(String),
    MissingParameter(String),

    GitHubError(github::GitHubError),
    NetworkError(reqwest::Error),
    OktaError(okta::OktaError),
    PagerDutyError(pagerduty::PagerDutyError),
    QuorumError(quorum::QuorumError),
    RusticaError(rustica::RusticaError),
    SlackError(slack::SlackError),
    SplunkError(splunk::SplunkError),
    YubikeyError(yubikey::YubikeyError),
    WebError(web::WebError),
}

impl Api {
    pub fn new(
        config: Apis,
        log_sender: Sender<Message>,
        delayed_log_sender: Sender<DelayedMessage>,
    ) -> Self {
        let general = config.general.map(|gc| General::new(gc, log_sender, delayed_log_sender));

        let github = config.github.map(Github::new);

        let okta = config.okta.map(Okta::new);

        let pagerduty = config.pagerduty.map(PagerDuty::new);

        #[cfg(feature = "quorum")]
        let quorum = match config.quorum {
            Some(q) => Some(Quorum::new(q)),
            _ => None,
        };
        #[cfg(not(feature = "quorum"))]
        let quorum = None;

        let rustica = config.rustica.map(Rustica::new);

        let slack = config.slack.map(Slack::new);

        let splunk = config.splunk.map(Splunk::new);

        let yubikey = config.yubikey.map(Yubikey::new);

        let web = config.web.map(Web::new);

        Self {
            runtime: Runtime::new().unwrap(),
            general,
            github,
            okta,
            pagerduty,
            quorum,
            rustica,
            slack,
            splunk,
            yubikey,
            web,
        }
    }
}
