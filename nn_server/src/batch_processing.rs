use std::{
    fmt::Debug,
    time::{Duration, Instant},
};

use tokio::{
    sync::{mpsc, oneshot},
    time::sleep,
};
use tracing_unwrap::ResultExt;

/// Commands for batch processing
#[derive(Debug)]
pub enum Command<In, Out> {
    /// Add item to current batch (to receive output through oneshot channel)
    Add((In, oneshot::Sender<Out>)),
    /// Process current batch
    Flush,
}

/// Start batch process with given settings and processing function, returns command sender
pub fn start_batch_process<In, Out, F>(
    batch_size: usize,
    max_delay: Duration,
    max_capacity: usize,
    process: F,
) -> mpsc::Sender<Command<In, Out>>
where
    In: Send + 'static,
    Out: Send + 'static,
    F: Fn(Vec<In>) -> Vec<Out> + Send + Copy + 'static,
{
    let (tx, mut rx) = mpsc::channel(max_capacity);
    // Start task for processing commands
    tokio::spawn(async move {
        // Current batch
        let mut queue = Vec::new();
        // Future for waiting until maximum delay
        let mut timeout = None;
        // Receive command or flush on timeout
        while let Some(command) = tokio::select! {
            _ = async { timeout.as_mut().unwrap().await }, if timeout.is_some() => Some(Command::Flush),
            x = rx.recv() => x,
        } {
            let need_flush = match command {
                Command::Add(x) => {
                    // Start waiting for other items
                    if queue.is_empty() {
                        timeout = Some(Box::pin(sleep(max_delay)));
                    }
                    queue.push(x);
                    // Flush when received full batch
                    queue.len() == batch_size
                }
                Command::Flush => true,
            };

            if need_flush {
                // Timeout is no longer needed
                timeout = None;
                if queue.is_empty() {
                    continue;
                }
                // Get current batch and split into inputs and senders
                let batch = std::mem::take(&mut queue);
                let (inputs, senders): (Vec<_>, Vec<_>) = batch.into_iter().unzip();
                // Process inputs
                let outputs = tokio::task::spawn_blocking(move || process(inputs))
                    .await
                    .unwrap_or_log();
                // Send all outputs
                for (sender, output) in senders.into_iter().zip(outputs) {
                    if sender.send(output).is_err() {
                        tracing::warn!("Receiver dropped before receiving batched result");
                    }
                }
            }
        }
    });
    tx
}

/// Send item to batch process, optionally send flush command, receive output
pub async fn batch_process<In: Debug, Out: Debug>(
    sender: &mpsc::Sender<Command<In, Out>>,
    value: In,
    flush: bool,
) -> Out {
    // Create channel for receiving output
    let (tx, rx) = oneshot::channel();
    // Send input
    sender
        .send(Command::Add((value, tx)))
        .await
        .expect_or_log("Error sending to batch processing channel");
    // Send flush command if needed
    if flush {
        sender
            .send(Command::Flush)
            .await
            .expect_or_log("Error sending to batch processing channel");
    }
    // Receive output
    rx.await
        .expect_or_log("Error receiving from batch processing channel")
}

/// Run processing function on batch and log model name, batch size and processing time
pub fn log_processing_function<In, Out, F>(
    name: &'static str,
    process: F,
    batch: Vec<In>,
) -> Vec<Out>
where
    F: Fn(Vec<In>) -> anyhow::Result<Vec<Out>>,
{
    let start_time = Instant::now();
    let res = process(batch).expect_or_log("Can't compute embedding");
    let indexing_duration = Instant::now() - start_time;
    tracing::info!(
        "{} processed {} requests in {:#?}",
        name,
        res.len(),
        indexing_duration
    );
    res
}
