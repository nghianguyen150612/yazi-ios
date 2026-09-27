use notify::Result;
use tokio::sync::mpsc;

use crate::{Refresher, Reporter, WATCHED, Watchee, local::{self, LINKED, Linked}, r#virtual};

pub(crate) struct Backend {
	local:               local::Local,
	r#virtual:           r#virtual::Virtual,
	pub(super) reporter: Reporter,
}

impl Backend {
	pub(crate) fn serve(refresher: Refresher) -> Self {
		#[cfg(any(target_os = "linux", target_os = "macos", target_os = "ios"))]
		yazi_fs::mounts::Partitions::monitor(&yazi_fs::mounts::PARTITIONS, || {
			crate::MgrProxy::watch();
			crate::MgrProxy::refresh();
			yazi_macro::log_if_err!(yazi_dds::Pubsub::pub_after_mount())
		});

		let (local_tx, local_rx) = mpsc::unbounded_channel();
		let (virtual_tx, virtual_rx) = mpsc::unbounded_channel();
		let reporter = Reporter { local_tx, virtual_tx };

		Self {
			local: local::Local::serve(local_rx, reporter.clone(), refresher),
			r#virtual: r#virtual::Virtual::serve(virtual_rx),
			reporter,
		}
	}

	pub(super) fn watch(&mut self, watchee: &mut Watchee) -> Result<()> {
		match watchee {
			Watchee::Local(..) => self.local.watch(watchee),
			Watchee::Virtual(_) => self.r#virtual.watch(watchee),
		}
	}

	pub(super) fn unwatch(&mut self, watchee: &Watchee) -> Result<()> {
		match watchee {
			Watchee::Local(..) => self.local.unwatch(watchee),
			Watchee::Virtual(_) => self.r#virtual.unwatch(watchee),
		}
	}

	pub(super) async fn sync(self) -> Self {
		Linked::sync(&LINKED, &WATCHED).await;
		self
	}
}
