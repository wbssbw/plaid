use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crossbeam_channel::Receiver;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Benchmarking {
    /// The full path to the output file where benchmarking metrics should be written
    #[serde(default = "default_results_file_path")]
    output_file_path: String,
}

/// Default file path for benchmarking results if none is provided in the config
fn default_results_file_path() -> String {
    format!(
        "{}/benchmark_results/metrics.txt",
        env!("CARGO_MANIFEST_DIR")
    )
}

/// Metadata about a rule's execution
pub struct ModulePerformanceMetadata {
    /// The name of the module
    pub module: String,
    /// Time (in microseconds) for execution to complete
    pub execution_time: u128,
    /// The amount of computation used by the rule
    pub computation_used: u64,
}

/// Represents a module's aggregate performance
struct AggregatePerformanceData {
    /// The number of times the module has been executed
    runs: u64,
    /// The total time (in microseconds) the module has spent in the execution loop
    total_execution_time: u128,
    /// The total computation used by the module
    total_computation_used: u64,
    /// Denotes whether the system should continue collecting performance metadata
    maxed_out: bool,
}

impl AggregatePerformanceData {
    /// Creates a new `AggregatePerformanceData` with initial execution time and computation used.
    fn new(execution_time: u128, computation_used: u64) -> Self {
        Self {
            runs: 1,
            total_execution_time: execution_time,
            total_computation_used: computation_used,
            maxed_out: false,
        }
    }

    /// Updates the aggregate data atomically with the latest execution time and computation used.
    /// If overflow occurs, the update is rolled back and none of the fields are modified.
    fn update(&mut self, message: &ModulePerformanceMetadata) {
        if self.maxed_out {
            return;
        }

        // Check if the additions would overflow before proceeding
        if let (Some(new_total_execution_time), Some(new_total_computation_used)) = (
            self.total_execution_time
                .checked_add(message.execution_time),
            self.total_computation_used
                .checked_add(message.computation_used),
        ) {
            // Atomic update
            self.runs += 1;
            self.total_execution_time = new_total_execution_time;
            self.total_computation_used = new_total_computation_used;
        } else {
            error!("Overflow occurred updating execution data for [{}]. No further execution data will be collected.", message.module);
            self.maxed_out = true;
        }
    }
}

impl Benchmarking {
    pub async fn benchmark_loop(
        &self,
        receiver: Receiver<ModulePerformanceMetadata>,
        shutdown_initiated: Arc<AtomicBool>,
    ) {
        let mut aggregate_performance_metadata = HashMap::new();

        // Benchmarking loop runs until shutdown_initiated is set to true
        while !shutdown_initiated.load(Ordering::SeqCst) {
            match receiver.try_recv() {
                Ok(message) => {
                    aggregate_performance_metadata
                        .entry(message.module.clone())
                        .and_modify(|aggregate: &mut AggregatePerformanceData| {
                            aggregate.update(&message);
                        })
                        .or_insert_with(|| {
                            AggregatePerformanceData::new(
                                message.execution_time,
                                message.computation_used,
                            )
                        });
                }
                Err(_) => {
                    // Sleep for a short period to avoid busy looping when no data is received
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }

        if let Err(e) =
            generate_report(&aggregate_performance_metadata, &self.output_file_path).await
        {
            error!("Failed to generate benchmark report. Error: {e}")
        }
    }
}

/// Generates a performance report based on the given aggregate performance data
/// and writes the results to the specified file.
async fn generate_report(
    aggregate_performance_metadata: &HashMap<String, AggregatePerformanceData>,
    file_path: &str,
) -> Result<(), tokio::io::Error> {
    debug!("Writing benchmarking results file to {file_path}...");

    // Open a file in write mode asynchronously. If the file doesn't exist, it will be created.
    let mut file = File::create(file_path).await?;

    // Write the report header
    file.write_all(b"Performance Report:\n").await?;

    // Write the data for each module
    for (module, data) in aggregate_performance_metadata {
        file.write_all(format!("Module: {}\n", module).as_bytes())
            .await?;
        file.write_all(format!("\tRuns: {}\n", data.runs).as_bytes())
            .await?;
        file.write_all(
            format!(
                "\tAverage Computation Used: {}\n",
                data.total_computation_used / data.runs
            )
            .as_bytes(),
        )
        .await?;
        file.write_all(
            format!(
                "\tAverage Execution Time (microseconds): {}\n",
                data.total_execution_time / data.runs as u128
            )
            .as_bytes(),
        )
        .await?;
        file.write_all(b"\n").await?;
    }

    // Ensure the file is flushed and fully written
    file.flush().await?;

    Ok(())
}
