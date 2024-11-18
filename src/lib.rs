#![cfg_attr(not(any(feature = "std", test)), no_std)]

pub struct R503 {
    address: u32,
}

impl R503 {
    pub fn new_with_address(addr: u32) -> Self {
        Self { address: addr }
    }
}
