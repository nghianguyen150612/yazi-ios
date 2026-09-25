pub struct Uzers;

impl Uzers {
	pub fn uid_or_zero() -> u32 { yazi_macro::unix_either!(Self::uid(), 0) }
}
