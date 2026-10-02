//! Fixed registry: callers cannot inject Store IDs or package identities.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Game { Dungeons, Bedrock }
impl Game {
    pub fn parse(id: &str) -> Result<Self,std::io::Error> { match id { "dungeons2"=>Ok(Self::Dungeons), "bedrock"=>Ok(Self::Bedrock), _=>Err(std::io::Error::other("Game invalid")) } }
    pub fn id(self)-> &'static str { match self { Self::Dungeons=>"dungeons2",Self::Bedrock=>"bedrock" } }
    pub fn store_id(self)-> &'static str { match self { Self::Dungeons=>"9P5786PJB9RP",Self::Bedrock=>"9NBLGGH2JHXJ" } }
    pub fn baseline(self)-> &'static str { match self { Self::Dungeons=>"1.1.1.0",Self::Bedrock=>"1.26.5203.0" } }
    pub fn validate(self,version:&str)->Result<(),std::io::Error> {
        let parts:Vec<_>=version.split('.').collect();
        if parts.len()!=4 || parts.iter().any(|s|s.is_empty()||!s.bytes().all(|b|b.is_ascii_digit())||s.parse::<u16>().is_err()) {return Err(std::io::Error::other("PACKAGE_VERSION_INVALID"))}
        let values:Vec<u16>=parts.iter().map(|s|s.parse().unwrap()).collect();
        let base:Vec<u16>=self.baseline().split('.').map(|s|s.parse().unwrap()).collect();
        if self==Self::Bedrock && values<base {return Err(std::io::Error::other("BEDROCK_VERSION_TOO_OLD"))}
        if values!=base {return Err(std::io::Error::other("BEDROCK_COMPATIBILITY_UNVERIFIED"))}Ok(())
    }
}
#[cfg(test)]mod tests{use super::*;#[test]fn registry_and_version_gate(){assert_eq!(Game::Bedrock.store_id(),"9NBLGGH2JHXJ");assert!(Game::Bedrock.validate("1.26.5203.0").is_ok());for v in ["1.26.5202.0","1.27.0.0","bad","1.26.5203","+1.26.5203.0"]{assert!(Game::Bedrock.validate(v).is_err());}assert!(Game::parse("../bedrock").is_err());}}
