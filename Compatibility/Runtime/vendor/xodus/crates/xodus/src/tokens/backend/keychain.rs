use crate::tokens::store::{TokenBackend, TokenStoreError};
pub struct KeychainBackend;
impl TokenBackend for KeychainBackend {
 fn get(&self,key:&str)->Result<Option<Vec<u8>>,TokenStoreError>{Ok(credential_bridge::client("get",key,None)?)}
 fn set(&self,key:&str,value:&[u8])->Result<(),TokenStoreError>{credential_bridge::client("set",key,Some(value.to_vec()))?;Ok(())}
 fn remove(&self,key:&str)->Result<(),TokenStoreError>{credential_bridge::client("remove",key,None)?;Ok(())}
}
