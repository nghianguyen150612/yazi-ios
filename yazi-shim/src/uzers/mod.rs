yazi_macro::mod_flat!(uzers);

#[cfg(all(unix, not(target_os = "ios")))]
yazi_macro::mod_flat!(unix);

#[cfg(target_os = "ios")]
yazi_macro::mod_flat!(ios);

#[cfg(all(unix, any(target_os = "ios", test)))]
mod lookup;
