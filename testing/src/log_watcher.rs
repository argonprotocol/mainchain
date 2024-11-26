use anyhow::bail;
use regex::Regex;
use std::{
	io::{BufRead, BufReader, Read},
	sync::{Arc, Mutex},
};
use tokio::{sync::watch, task};

#[derive(Clone)]
pub struct LogWatcher {
	log: Arc<Mutex<String>>,
	log_receiver: watch::Receiver<()>,
	handle: Arc<Mutex<Option<task::JoinHandle<()>>>>,
}

impl LogWatcher {
	pub fn new<R: Read + Send + Sync + 'static>(log_in: R) -> Self {
		let (log_notifier, log_receiver) = watch::channel(());
		let log = Arc::new(Mutex::new(String::new()));
		let stderr_reader = BufReader::new(log_in);

		let log_clone = log.clone();

		let handle = task::spawn_blocking(move || {
			for line in stderr_reader.lines() {
				let line = line.expect("failed to obtain next line from stdout");

				{
					let mut log_guard = log_clone.lock().unwrap();
					log_guard.push_str(&line);
					log_guard.push('\n');
				}

				let _ = log_notifier.send(()); // Notify log updates
			}
		});
		Self { log, handle: Arc::new(Mutex::new(Some(handle))), log_receiver }
	}

	pub fn close(&self) {
		if let Some(handle) = self.handle.lock().unwrap().take() {
			handle.abort();
		}
	}

	// Wait for a specific regex to match a number of times in the log
	pub async fn wait_for_log(&self, pattern: &str, count: usize) -> anyhow::Result<Vec<String>> {
		let regex = Regex::new(pattern)?;
		let mut log_receiver = self.log_receiver.clone(); // Watch receiver for updates

		loop {
			// Check the current log for matches
			{
				let log_guard = self.log.lock().unwrap();
				let matches = regex.captures(&log_guard);
				if let Some(matches) = matches {
					if matches.len() >= count {
						let mut results = Vec::new();
						for (i, m) in matches.iter().enumerate() {
							if i == 0 {
								continue;
							}
							if let Some(m) = m {
								results.push(m.as_str().to_string());
							}
						}
						return Ok(results);
					}
				}
			}

			// Wait for the next log update or timeout
			if log_receiver.changed().await.is_err() {
				bail!("Log watcher closed");
			}
		}
	}
}
