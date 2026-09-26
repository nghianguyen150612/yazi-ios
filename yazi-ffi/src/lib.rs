yazi_macro::mod_pub!(shm);

#[cfg(target_os = "macos")]
yazi_macro::mod_flat!(cf_dict cf_string disk_arbitration io_kit);

#[cfg(target_os = "ios")]
yazi_macro::mod_pub!(pasteboard);

#[cfg(windows)]
yazi_macro::mod_flat!(com);
