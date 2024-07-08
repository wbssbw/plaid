pub mod github;
mod internal;
mod interval;
mod okta;
use crate::{
    executor::Message,
    storage::{Storage, StorageError},
};

use std::{sync::Arc, time::Duration};

use crossbeam_channel::Sender;

use serde::Deserialize;

pub use self::internal::DelayedMessage;

// Configure data sources that Plaid will use fetch data itself and
// send to modules
#[derive(Deserialize)]
pub struct DataConfig {
    github: Option<github::GithubConfig>,
    okta: Option<okta::OktaConfig>,
    internal: Option<internal::InternalConfig>,
    interval: Option<interval::IntervalConfig>,
}

struct DataInternal {
    github: Option<github::Github>,
    okta: Option<okta::Okta>,
    // Perhaps in the future there will be a reason to explicitly disallow
    // sending logs from one rule to another but for now we keep it always
    // enabled.
    internal: Option<internal::Internal>,
    /// Interval manages tracking and execution of jobs that are executed on a defined interval
    interval: Option<interval::Interval>,
}

pub struct Data {}

#[derive(Debug)]
pub enum DataError {
    StorageError(StorageError),
}

impl DataInternal {
    async fn new(
        config: DataConfig,
        logger: Sender<Message>,
        storage: Option<Arc<Storage>>,
    ) -> Result<Self, DataError> {
        let github = config.github.map(|gh| github::Github::new(gh, logger.clone()));

        let okta = config.okta.map(|okta| okta::Okta::new(okta, logger.clone()));

        let internal = if let Some(internal) = config.internal {
            internal::Internal::new(internal, logger.clone(), storage.clone()).await
        } else {
            internal::Internal::new(
                internal::InternalConfig::default(),
                logger.clone(),
                storage.clone(),
            )
            .await
        };

        let interval = config
            .interval
            .map(|config| interval::Interval::new(config, logger.clone()));

        Ok(Self {
            github,
            okta,
            internal: Some(internal?),
            interval,
        })
    }
}

impl Data {
    pub async fn start(
        config: DataConfig,
        sender: Sender<Message>,
        storage: Option<Arc<Storage>>,
    ) -> Result<Option<Sender<DelayedMessage>>, DataError> {
        let di = DataInternal::new(config, sender, storage).await?;
        let handle = tokio::runtime::Handle::current();

        // Start the Github Audit task if there is one
        if let Some(mut gh) = di.github {
            handle.spawn(async move {
                loop {
                    if let Err(e) = gh.fetch_audit_logs().await {
                        error!("GitHub Data Fetch Error: {}", e)
                    }

                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            });
        }

        // Start the Okta System Logs task if there is one
        if let Some(mut okta) = di.okta {
            handle.spawn(async move {
                loop {
                    if let Err(e) = okta.fetch_system_logs().await {
                        error!("Okta Data Fetch Error: {:?}", e)
                    }

                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            });
        }

        let internal_sender = di.internal.as_ref().map(|internal| internal.get_sender());

        // Start the interval job processor
        if let Some(mut interval) = di.interval {
            handle.spawn(async move {
                loop {
                    let time_until_next_execution = interval.fetch_interval_jobs().await;
                    tokio::time::sleep(Duration::from_secs(time_until_next_execution)).await;
                }
            });
        }

        // Start the internal log processor. This doesn't need to be a tokio task,
        // but we make it one incase we need the runtime in the future. Perhaps it
        // will make sense to convert it to a standard thread but I don't see a benefit
        // to that now. As long as we don't block.
        if let Some(mut internal) = di.internal {
            handle.spawn(async move {
                loop {
                    if let Err(e) = internal.fetch_internal_logs().await {
                        error!("Internal Data Fetch Error: {:?}", e)
                    }

                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            });
        }
        info!("Started Data collection tasks");
        Ok(internal_sender)
    }
}
