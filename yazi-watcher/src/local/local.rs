use std::{io, path::Path, time::Duration};

use hashbrown::HashSet;
use notify::{PollWatcher, RecommendedWatcher, RecursiveMode, Result, Watcher};
use tokio::{pin, sync::mpsc::{self, UnboundedReceiver}};
use tokio_stream::{StreamExt, wrappers::UnboundedReceiverStream};
use yazi_fs::{FilesOp, engine::{self, Engine}, file::File, mounts::PARTITIONS};
use yazi_macro::error;
use yazi_shared::url::UrlLike;

use crate::{LocalReport, RefreshRequest, Refresher, Reporter, WATCHER, Watchee};

trait LocalWatcher: Send {
	fn watch(&mut self, path: &Path, mode: RecursiveMode) -> Result<()>;
	fn unwatch(&mut self, path: &Path) -> Result<()>;
}

impl<T: Watcher + Send> LocalWatcher for T {
	fn watch(&mut self, path: &Path, mode: RecursiveMode) -> Result<()> {
		<T as Watcher>::watch(self, path, mode)
	}

	fn unwatch(&mut self, path: &Path) -> Result<()> {
		<T as Watcher>::unwatch(self, path)
	}
}

pub(crate) struct Local {
	primary:     Option<Box<dyn LocalWatcher>>,
	alternative: Option<Box<dyn LocalWatcher>>,
}

impl Local {
	pub(crate) fn serve(
		rx: mpsc::UnboundedReceiver<LocalReport>,
		reporter: Reporter,
		refresher: Refresher,
	) -> Self {
		tokio::spawn(Self::changed(rx, refresher));

		let config = notify::Config::default().with_poll_interval(Duration::from_secs(1));
		let handler = move |res: Result<notify::Event>| match res {
			Ok(event) if !event.kind.is_access() => reporter.report(event.paths),
			Ok(_) => (),
			Err(e) => error!("Filesystem watcher reported an error: {e:?}"),
		};

		let primary = if cfg!(target_os = "ios") {
			yazi_macro::warn!(
				"Using PollWatcher for iOS local paths: nonrecursive kqueue watches do not report child content changes or follow replaced paths"
			);
			None
		} else {
			match RecommendedWatcher::new(handler.clone(), config) {
				Ok(watcher) => Some(Box::new(watcher) as Box<dyn LocalWatcher>),
				Err(e) => {
					error!("Failed to initialize primary watcher: {e:?}");
					None
				}
			}
		};
		let alternative =
			PollWatcher::new(handler, config).map(|watcher| Box::new(watcher) as Box<dyn LocalWatcher>);

		if let Err(e) = &alternative {
			error!("Failed to initialize polling watcher: {e:?}");
		}

		Self { primary, alternative: alternative.ok() }
	}

	pub(crate) fn watch(&mut self, watchee: &mut Watchee) -> Result<()> {
		let (path, alt) =
			watchee.as_local_mut().ok_or_else(|| notify::Error::generic("Not a local watchee"))?;

		if let Some(primary) = self.primary.as_mut().filter(|_| !*alt) {
			match primary.watch(path, RecursiveMode::NonRecursive) {
				Ok(()) => return Ok(()),
				Err(e) => {
					yazi_macro::warn!("Failed to watch {path:?} with primary watcher: {e:?}");
					if let Err(cleanup) = primary.unwatch(path)
						&& !matches!(cleanup.kind, notify::ErrorKind::WatchNotFound)
					{
						yazi_macro::warn!("Failed to clean up primary watch for {path:?}: {cleanup:?}");
					}
				}
			}
		}

		yazi_macro::debug!("Watching {path:?} with alternative watcher");
		*alt = true;
		self
			.alternative
			.as_mut()
			.ok_or_else(|| notify::Error::generic("Polling watcher is unavailable"))?
			.watch(path, RecursiveMode::NonRecursive)
	}

	pub(crate) fn unwatch(&mut self, watchee: &Watchee) -> Result<()> {
		let (path, alt) =
			watchee.as_local().ok_or_else(|| notify::Error::generic("Not a local watchee"))?;

		let result = if alt {
			match self.alternative.as_mut() {
				Some(alternative) => alternative.unwatch(path),
				None => Ok(()),
			}
		} else if let Some(primary) = &mut self.primary {
			primary.unwatch(path)
		} else {
			Ok(())
		};

		match result {
			Ok(()) => Ok(()),
			Err(e) if matches!(e.kind, notify::ErrorKind::WatchNotFound) => Ok(()),
			Err(e) => Err(e)?,
		}
	}

	pub(crate) async fn use_alternative(path: &Path) -> bool {
		if Self::requires_alternative(false, cfg!(target_os = "ios"))
			|| cfg!(target_os = "netbsd")
			|| yazi_adapter::WSL.get()
		{
			return true;
		}

		match engine::local::Local::regular(path).metadata().await {
			Ok(cha) => Self::requires_alternative(PARTITIONS.read().soundless(cha), false),
			Err(_) => true,
		}
	}

	fn requires_alternative(soundless: bool, is_ios: bool) -> bool {
		soundless || is_ios
	}

	async fn changed(rx: UnboundedReceiver<LocalReport>, refresher: Refresher) {
		// TODO: revert this once a new notification is implemented
		let rx = UnboundedReceiverStream::new(rx).chunks_timeout(1000, Duration::from_millis(250));
		pin!(rx);

		while let Some(chunk) = rx.next().await {
			let urls: HashSet<_> = chunk.into_iter().collect();

			let _permit = WATCHER.acquire().await.unwrap();
			let mut ops = Vec::with_capacity(urls.len());

			for url in urls {
				let (url, is_event_path) = match url {
					LocalReport::Url(url) => (url, true),
					LocalReport::Trail(url) => (url, false),
				};
				let Some(path) = url.as_local() else { continue };
				let Some((trail, key)) = url.pair() else { continue };

				let file = match engine::local::Local::regular(path).file().await {
					Ok(file) => file,
					Err(e) if e.kind() == io::ErrorKind::NotFound => {
						ops.push(FilesOp::Deleting(trail.into(), [key.into()].into()));
						continue;
					}
					Err(e) => {
						yazi_macro::error!("Failed to update {url}: {e:?}");
						continue;
					}
				};

				if Self::should_rescan_directory(is_event_path, &file) {
					refresher.refresh([RefreshRequest::force(file)]);
					continue;
				}

				if !engine::local::match_name_case(path).await {
					ops.push(FilesOp::Deleting(trail.into(), [key.into()].into()));
					continue;
				}

				ops.push(FilesOp::Upserting(trail.into(), [(key.into(), file)].into()));
			}

			FilesOp::mutate(ops);
		}
	}

	fn should_rescan_directory(is_event_path: bool, file: &File) -> bool {
		is_event_path && file.is_dir()
	}
}

#[cfg(test)]
mod tests {
	use std::{
		path::{Path, PathBuf},
		sync::{Arc, Mutex, mpsc::{self, Receiver}},
		time::{Duration, Instant, SystemTime, UNIX_EPOCH},
	};

	use notify::{Error, Event, PollWatcher, RecursiveMode, Watcher};
	use yazi_fs::{cha::ChaType, file::File};

	use super::{Local, LocalWatcher};
	use crate::Watchee;

	#[derive(Clone)]
	struct FakeWatcher {
		name: &'static str,
		calls: Arc<Mutex<Vec<(&'static str, &'static str)>>>,
		watch_fails: bool,
		unwatch_fails: bool,
	}

	impl LocalWatcher for FakeWatcher {
		fn watch(&mut self, _path: &Path, _mode: RecursiveMode) -> notify::Result<()> {
			self.calls.lock().unwrap().push((self.name, "watch"));
			if self.watch_fails { Err(Error::generic("scripted watch failure")) } else { Ok(()) }
		}

		fn unwatch(&mut self, _path: &Path) -> notify::Result<()> {
			self.calls.lock().unwrap().push((self.name, "unwatch"));
			if self.unwatch_fails { Err(Error::watch_not_found()) } else { Ok(()) }
		}
	}

	fn fake(
		name: &'static str,
		calls: &Arc<Mutex<Vec<(&'static str, &'static str)>>>,
		watch_fails: bool,
	) -> Box<dyn LocalWatcher> {
		Box::new(FakeWatcher { name, calls: calls.clone(), watch_fails, unwatch_fails: false })
	}

	fn watchee(path: &str, alt: bool) -> Watchee<'static> {
		Watchee::Local(PathBuf::from(path).into(), alt)
	}

	fn calls() -> Arc<Mutex<Vec<(&'static str, &'static str)>>> {
		Arc::new(Mutex::new(Vec::new()))
	}

	#[test]
	fn primary_registration_keeps_primary_state() {
		let calls = calls();
		let mut backend = Local {
			primary: Some(fake("primary", &calls, false)),
			alternative: Some(fake("alternative", &calls, false)),
		};
		let mut watched = watchee("/tmp/primary", false);

		assert!(backend.watch(&mut watched).is_ok());
		assert!(matches!(watched, Watchee::Local(_, false)));
		assert_eq!(*calls.lock().unwrap(), [("primary", "watch")]);
	}

	#[test]
	fn primary_failure_cleans_up_and_uses_alternative() {
		let calls = calls();
		let mut backend = Local {
			primary: Some(fake("primary", &calls, true)),
			alternative: Some(fake("alternative", &calls, false)),
		};
		let mut watched = watchee("/tmp/fallback", false);

		assert!(backend.watch(&mut watched).is_ok());
		assert!(matches!(watched, Watchee::Local(_, true)));
		assert_eq!(
			*calls.lock().unwrap(),
			[("primary", "watch"), ("primary", "unwatch"), ("alternative", "watch")]
		);
	}

	#[test]
	fn unavailable_primary_uses_alternative() {
		let calls = calls();
		let mut backend =
			Local { primary: None, alternative: Some(fake("alternative", &calls, false)) };
		let mut watched = watchee("/tmp/no-primary", false);

		assert!(backend.watch(&mut watched).is_ok());
		assert!(matches!(watched, Watchee::Local(_, true)));
		assert_eq!(*calls.lock().unwrap(), [("alternative", "watch")]);
	}

	#[test]
	fn alternative_failure_propagates_and_keeps_alt_state() {
		let calls = calls();
		let mut backend = Local { primary: None, alternative: Some(fake("alternative", &calls, true)) };
		let mut watched = watchee("/tmp/no-watcher", false);

		assert!(backend.watch(&mut watched).is_err());
		assert!(matches!(watched, Watchee::Local(_, true)));
	}

	#[test]
	fn unavailable_alternative_returns_an_error_without_panicking() {
		let mut backend = Local { primary: None, alternative: None };
		let mut watched = watchee("/tmp/no-backends", false);

		assert!(backend.watch(&mut watched).is_err());
		assert!(matches!(watched, Watchee::Local(_, true)));
	}

	#[test]
	fn unwatch_uses_the_registered_backend_and_ignores_watch_not_found() {
		let calls = calls();
		let primary = FakeWatcher {
			name: "primary",
			calls: calls.clone(),
			watch_fails: false,
			unwatch_fails: true,
		};
		let mut backend = Local {
			primary: Some(Box::new(primary.clone())),
			alternative: Some(fake("alternative", &calls, false)),
		};

		assert!(backend.unwatch(&watchee("/tmp/primary", false)).is_ok());
		assert!(backend.unwatch(&watchee("/tmp/alternative", true)).is_ok());
		assert_eq!(*calls.lock().unwrap(), [("primary", "unwatch"), ("alternative", "unwatch")]);
		assert!(primary.unwatch_fails);
	}

	#[test]
	fn ios_local_paths_use_poll_and_other_targets_keep_their_mount_policy() {
		assert!(Local::requires_alternative(false, true));
		assert!(Local::requires_alternative(true, false));
		assert!(!Local::requires_alternative(false, false));
	}

	#[test]
	fn directory_events_refresh_contents_but_file_events_upsert_metadata() {
		let directory = File::from_dummy(PathBuf::from("/tmp/watcher-dir"), Some(ChaType::Dir));
		let file = File::from_dummy(PathBuf::from("/tmp/watcher-file"), Some(ChaType::File));

		assert!(Local::should_rescan_directory(true, &directory));
		assert!(!Local::should_rescan_directory(false, &directory));
		assert!(!Local::should_rescan_directory(true, &file));
	}

	fn wait_for_path_event(
		receiver: &Receiver<notify::Result<Event>>,
		path: &Path,
		is_create: bool,
	) -> bool {
		let deadline = Instant::now() + Duration::from_secs(3);
		while Instant::now() < deadline {
			match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
				Ok(Ok(event))
					if event.paths.iter().any(|p| p == path)
						&& if is_create { event.kind.is_create() } else { event.kind.is_remove() } =>
				{
					return true;
				}
				Ok(_) => (),
				Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => break,
			}
		}
		false
	}

	#[test]
	fn poll_watcher_detects_delete_and_same_path_recreation() {
		let root = std::env::temp_dir().join(format!(
			"yazi-watcher-{}-{}",
			std::process::id(),
			SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
		));
		std::fs::create_dir(&root).unwrap();
		let path = root.join("watched-file");
		std::fs::write(&path, b"old").unwrap();
		let (tx, rx) = mpsc::channel();
		let config = notify::Config::default().with_poll_interval(Duration::from_millis(25));
		let mut watcher = PollWatcher::new(
			move |event| {
				tx.send(event).ok();
			},
			config,
		)
		.unwrap();
		Watcher::watch(&mut watcher, &path, RecursiveMode::NonRecursive).unwrap();

		std::fs::remove_file(&path).unwrap();
		let removed = wait_for_path_event(&rx, &path, false);
		std::fs::write(&path, b"new inode contents").unwrap();
		let recreated = wait_for_path_event(&rx, &path, true);

		drop(watcher);
		std::fs::remove_dir_all(root).unwrap();
		assert!(removed, "PollWatcher did not report removal of the watched path");
		assert!(recreated, "PollWatcher did not report same-path recreation");
	}
}
