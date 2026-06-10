use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use crate::traits::HashVerifier;

pub struct DefaultHashVerifier;

impl HashVerifier for DefaultHashVerifier {
    fn verify(&self, data: &[u8], expected_hash: &str) -> bool {
        let actual = self.calculate_hash(data);
        actual == expected_hash
    }

    fn calculate_hash(&self, data: &[u8]) -> String {
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}
