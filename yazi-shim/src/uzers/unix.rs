use super::Uzers;

static USERS_CACHE: crate::cell::RoCell<::uzers::UsersCache> = crate::cell::RoCell::new();

impl Uzers {
	pub(crate) fn init() {
		USERS_CACHE.with(<_>::default);
	}

	pub fn uid() -> u32 {
		use ::uzers::Users;
		USERS_CACHE.get_current_uid()
	}

	pub fn gid() -> u32 {
		use ::uzers::Groups;
		USERS_CACHE.get_current_gid()
	}

	pub fn user_name(uid: Option<u32>) -> Option<std::ffi::OsString> {
		use ::uzers::Users;
		USERS_CACHE
			.get_user_by_uid(uid.unwrap_or_else(Self::uid))
			.map(|u| u.name().to_owned())
	}

	pub fn group_name(gid: Option<u32>) -> Option<std::ffi::OsString> {
		use ::uzers::Groups;
		USERS_CACHE
			.get_group_by_gid(gid.unwrap_or_else(Self::gid))
			.map(|g| g.name().to_owned())
	}

	pub fn home_dir() -> Option<std::path::PathBuf> {
		use ::uzers::{Users, os::unix::UserExt};
		USERS_CACHE
			.get_user_by_uid(Self::uid())
			.map(|u| u.home_dir().to_owned())
			.filter(|p| p.is_absolute())
	}
}
