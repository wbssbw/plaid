use clap::{Arg, Command};

use plaid_stl::messages::LogbacksAllowed;
use serde::{de, Deserialize};
use serde_json::Value;

use std::collections::HashMap;

use super::apis::Apis;
use super::data::DataConfig;
use super::loader::Configuration as LoaderConfiguration;
use super::logging::LoggingConfiguration;
use super::storage::Config as StorageConfig;

/// How should responses to GET requests be cached.
#[derive(Default, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum CachingMode {
    /// Do not cache.
    #[default]
    None,
    /// Run then cache the result for the given period of time. The
    /// cache will be invalidated after the given period of time has
    /// passed.
    Timed { validity: u64 },
    /// This will use a stored cache from whatever the response method is
    /// before running the response method again. This means that GET calls
    /// should always be fast as the cache is only updated by other means.
    ///
    /// The exception is if call_on_none is set to true, in which case if
    /// nothing is stored, this will act as if Cache::None was set for the first
    /// call.
    UsePersistentResponse { call_on_none: bool },
}

/// How should a webhook respond to a GET request
#[derive(Clone)]
pub enum ResponseMode {
    /// Respond the way Facebook expects, needs a secret token
    Facebook(String),
    /// Respond by running a plaid wasm module to generate the response
    Rule(String),
    /// Static response
    Static(String),
}

/// Some services have verification routines that need to happen before
/// data will be sent to them. To make the system more general you can
/// specify on each webhook what kind of response you'd like to use
/// to pass this verification.
///
/// This configuration controls how a webserver will respond to GET
/// requests.
#[derive(Deserialize, Clone)]
pub struct GetMode {
    /// Set how the data sent in GET responses should be cached. This is really
    /// only useful when the response_mode is set to ResponseMode::Rule but in future
    /// this may be applicable to other, newer, modes.
    #[serde(default)]
    pub caching_mode: CachingMode,
    #[serde(deserialize_with = "response_mode_deserializer")]
    pub response_mode: ResponseMode,
}

/// Configuration for a particular webhook within a WebhookServer to accept
/// logs and send them to a logging channel
#[derive(Deserialize, Clone)]
pub struct WebhookConfig {
    /// The logging channel that POST bodies will be sent to
    pub log_type: String,
    /// What headers do you want forwarded to the logging channel as well under
    /// accessory data. If these headers conflict with set secrets, the secrets
    /// will be used and the header ignored.
    pub headers: Vec<String>,
    /// See GetMode
    pub get_mode: Option<GetMode>,
    /// An optional label for the webhook. If this is populated, it will be
    /// passed as the source to to the modules instead of the webhook address.
    /// You may want to do this to reduce the secrets modules have access to.
    pub label: Option<String>,
    /// The maximum number of logbacks that each rule will be allowed to trigger
    /// per message received. If this is set to Limited(0), no rule will be able to use the log
    /// back functionality from messages generated by this webhook. If this is set to Limited(1),
    /// then EACH RULE will be able to trigger one logback. If this is set to Unlimited, then
    /// each rule will be able to trigger as many logbacks as they want (and those triggered rules
    /// will be able to as well). If this is not set, it will default to Limited(0).
    #[serde(default)]
    pub logbacks_allowed: LogbacksAllowed,
}

#[derive(Deserialize)]
pub struct WebhookServerConfiguration {
    /// The address and port to listen on for webhooks
    pub listen_address: String,
    /// The mapping of webhooks to configuration of the webhook
    #[serde(default)]
    pub webhooks: HashMap<String, WebhookConfig>,
}

/// The full configuration of Plaid
#[derive(Deserialize)]
pub struct Configuration {
    /// How APIs are configured. These APIs are accessible to modules
    /// so they can take advantage of Plaid abstractions
    pub apis: Apis,
    /// Data generators. These are systems that pull data directly rather
    /// than waiting for data to come in via Webhook
    pub data: DataConfig,
    /// How many threads should be used for executing modules when logs come in
    ///
    /// Modules do not get more than one thread, this just means that modules can
    /// execute in parallel
    pub execution_threads: u8,
    /// The maximum number of logs in the queue to be processed at once
    #[serde(default = "default_log_queue_size")]
    pub log_queue_size: usize,
    /// Configuration for persistent data. This allows modules to store data between
    /// invocations
    pub storage: Option<StorageConfig>,
    /// The external logging system. This allows you to send data to external systems
    /// for monitoring
    pub logging: LoggingConfiguration,
    /// See WebhookServerConfiguration
    pub webhooks: HashMap<String, WebhookServerConfiguration>,
    /// Set what modules will be loaded, what logging channels they're going to use
    /// and their computation and memory limits.
    pub loading: LoaderConfiguration,
}

/// This function provides the default log queue size in the event that one isn't provided
fn default_log_queue_size() -> usize {
    2048
}

#[derive(Debug)]
pub enum ConfigurationError {
    FileError,
    ParsingError,
    ComputationLimitInvalid,
    ExecutionThreadsInvalid,
}

impl std::fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigurationError::FileError => write!(
                f,
                "There was an error finding or reading the configuration file"
            ),
            ConfigurationError::ParsingError => {
                write!(f, "The format of the configuration file was incorrect")
            }
            ConfigurationError::ComputationLimitInvalid => {
                write!(f, "The computation limit must be non zero")
            }
            ConfigurationError::ExecutionThreadsInvalid => {
                write!(
                    f,
                    "The number of execution threads must be between 1 and 255"
                )
            }
        }
    }
}

impl std::error::Error for ConfigurationError {}

fn response_mode_deserializer<'de, D>(deserializer: D) -> Result<ResponseMode, D::Error>
where
    D: de::Deserializer<'de>,
{
    let mode = String::deserialize(deserializer)?;

    let mut pieces: Vec<&str> = mode.split(":").collect();

    let data = pieces.pop().ok_or(serde::de::Error::custom(
        "Must provide context for the response_mode. For Facebook/Meta this is the secret, for Rule this is the module name",
    ))?;

    let mode = pieces
        .pop()
        .ok_or(serde::de::Error::custom("Must provide a response_mode"))?;

    Ok(match mode {
        "facebook" | "meta" => ResponseMode::Facebook(data.to_owned()),
        "Rule" | "rule" => ResponseMode::Rule(data.to_owned()),
        "Static" | "static" => ResponseMode::Static(data.to_owned()),
        x => {
            return Err(serde::de::Error::custom(format!(
                "{x} is an unknown response_mode. Must be 'facebook', 'rule', or 'static'"
            )))
        }
    })
}
pub async fn configure() -> Result<Configuration, ConfigurationError> {
    let matches = Command::new("Plaid - A sandboxed automation engine")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Mitchell Grenier <mitchell@confurious.io>")
        .about("Write security rules in anything that compiles to WASM, run them with only the access they need.")
        .arg(
            Arg::new("config")
                .help("Path to the configuration toml file")
                .long("config")
                .default_value("./plaid/resources/plaid.toml")
        )
        .arg(
            Arg::new("secrets")
                .help("Path to the secrets json file")
                .long("secrets")
                .default_value("./plaid/private-resources/secrets.json")
        ).get_matches();

    // Read the configuration file
    let mut config =
        match tokio::fs::read_to_string(matches.get_one::<String>("config").unwrap()).await {
            Ok(config) => config,
            Err(e) => {
                error!("Encountered file error when trying to read configuration!. Error: {e}");
                return Err(ConfigurationError::FileError);
            }
        };

    // Read the secrets file and parse into a serde object
    let secrets = match tokio::fs::read(matches.get_one::<String>("secrets").unwrap()).await {
        Ok(secret_bytes) => {
            let secrets = serde_json::from_slice::<Value>(&secret_bytes).unwrap();
            secrets.as_object().cloned().unwrap()
        }
        Err(e) => {
            error!("Encountered file error when trying to read secrets file!. Error: {e}");
            return Err(ConfigurationError::FileError);
        }
    };

    // Iterate over the secrets we just parsed and replace matching keys in the config
    for (secret, value) in secrets {
        config = config.replace(&secret, value.as_str().unwrap());
    }

    // Parse the TOML into our configuration structures
    let config: Configuration = match toml::from_str(&config) {
        Ok(config) => config,
        Err(e) => {
            error!("Encountered parsing error while reading configuration with interpolated secrets!. Error: {e}");
            return Err(ConfigurationError::ParsingError);
        }
    };

    if config.execution_threads == 0 {
        return Err(ConfigurationError::ExecutionThreadsInvalid);
    }

    Ok(config)
}
