yazi_macro::mod_flat!(entries entry lua trash_id);

#[cfg(trash_unix)]
yazi_macro::mod_flat!(common);

#[cfg(target_os = "macos")]
yazi_macro::mod_flat!(macos);

#[cfg(windows)]
yazi_macro::mod_flat!(windows);

#[cfg(all(trash_freedesktop, not(target_os = "ios")))]
yazi_macro::mod_flat!(freedesktop);

#[cfg(target_os = "ios")]
yazi_macro::mod_flat!(ios);

#[cfg(all(test, not(target_os = "ios")))]
pub mod ios;

#[cfg(trash_unsupported)]
yazi_macro::mod_flat!(unsupported);

#[cfg(not(trash_unsupported))]
yazi_macro::mod_flat!(traits);
