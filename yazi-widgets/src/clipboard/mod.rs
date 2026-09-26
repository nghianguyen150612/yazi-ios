yazi_macro::mod_flat!(clipboard);

#[cfg(unix)]
mod lookup;

#[cfg(all(unix, not(target_os = "ios")))]
yazi_macro::mod_flat!(unix);

#[cfg(windows)]
yazi_macro::mod_flat!(windows);

// The iOS backend is also compiled by host tests, so its selection and
// encoding rules can be exercised without UIKit.
#[cfg(any(target_os = "ios", test))]
yazi_macro::mod_flat!(ios);
