//! Fixed registry: callers cannot inject Store IDs or package identities.
pub fn validate(self, version: &str) -> Result<(), std::io::Error> {
    let parts: Vec<_> = version.split('.').collect();
    if parts.len() != 4 || parts.iter().any(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) || s.parse::<u16>().is_err()) {
        return Err(std::io::Error::other("PACKAGE_VERSION_INVALID"))
    }
    
    let values: Vec<u16> = parts.iter().map(|s| s.parse().unwrap()).collect();
    
    let approved = match self {
        Self::Dungeons => vec![vec![1, 1, 1, 0], vec![1, 1, 2, 0]],
        Self::Bedrock => vec![vec![1, 26, 5203, 0]],
    };
    
    if approved.contains(&values) {
        Ok(())
    } else {
        Err(std::io::Error::other("COMPATIBILITY_UNVERIFIED"))
    }
}
