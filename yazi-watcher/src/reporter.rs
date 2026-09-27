use tokio::sync::mpsc;
use yazi_shared::url::{AsUrl, Url, UrlBuf, UrlCow, UrlLike};

use crate::{Watched, WATCHED, local::LINKED, r#virtual::VirtualReport};

#[derive(Clone)]
pub(crate) struct Reporter {
	pub(super) local_tx:   mpsc::UnboundedSender<LocalReport>,
	pub(super) virtual_tx: mpsc::UnboundedSender<VirtualReport>,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub(crate) enum LocalReport {
	Url(UrlBuf),
	Trail(UrlBuf),
}

impl Reporter {
	pub(crate) fn report<'a, I>(&self, urls: I)
	where
		I: IntoIterator,
		I::Item: Into<UrlCow<'a>>,
	{
		for url in urls.into_iter().map(Into::into) {
			if url.is_regular() {
				self.report_local(url);
			} else {
				self.report_virtual(url);
			}
		}
	}

	fn report_local(&self, url: UrlCow) {
		let Some((trail, _)) = url.pair() else { return };

		let linked = LINKED.read();
		let linked = linked.from_dir(trail).chain(linked.from_dir(url.as_url())).map(Url::regular);

		let watched = WATCHED.read();
		if Self::is_watched(&watched, url.as_url()) {
			self.local_tx.send(LocalReport::Url(url.to_owned())).ok();
		}

		for trail in [trail].into_iter().chain(linked) {
			if Self::is_watched(&watched, trail) {
				self.local_tx.send(LocalReport::Url(url.to_owned())).ok();
				self.local_tx.send(LocalReport::Trail(trail.to_owned())).ok();
			}

			if url.urn().ext().is_some_and(|e| e == "%tmp") {
				continue;
			}

			// Virtual caches
			let Some(dir) = watched.find_by_cache(trail.loc()) else { continue };
			let Some(key) = url.name() else { continue };
			self.virtual_tx.send(VirtualReport::Cache(dir, key.to_owned())).ok();
		}
	}

	fn is_watched(watched: &Watched, url: Url<'_>) -> bool {
		watched.contains_url(url)
	}

	fn report_virtual(&self, url: UrlCow) {
		let Some((trail, _)) = url.pair() else { return };
		if !WATCHED.read().contains_url(trail) {
			return;
		}

		self.virtual_tx.send(VirtualReport::Url(trail.to_owned())).ok();
		self.virtual_tx.send(VirtualReport::Url(url.into_owned())).ok();
	}
}

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use yazi_shared::url::{AsUrl, UrlBuf};

	use crate::{Watched, Watchee};

	use super::Reporter;

	#[test]
	fn reports_an_event_for_a_directly_watched_path_without_a_watched_parent() {
		let mut watched = Watched::default();
		let url = UrlBuf::from(PathBuf::from("/tmp/watched-directory"));
		watched.insert(Watchee::Local(url.clone().into(), false));

		assert!(Reporter::is_watched(&watched, url.as_url()));
		assert!(!Reporter::is_watched(&watched, url.as_url().parent().unwrap()));
	}
}
