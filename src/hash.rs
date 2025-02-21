use std::hash::{Hash, Hasher};

use siphasher::sip::SipHasher;

pub fn hash_to_str<T: Hash>(t: &T) -> String {
    let mut s = SipHasher::new();
    t.hash(&mut s);
    format!("{:x}", s.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_to_str() {
        assert_eq!(hash_to_str(&"foo"), "e1b19adfb2e348a2");
    }
}
