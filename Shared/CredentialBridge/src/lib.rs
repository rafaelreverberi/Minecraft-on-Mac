//! Bounded authenticated encryption for ephemeral loopback IPC. Never persist keys or messages.
use chacha20poly1305::{ChaCha20Poly1305, KeyInit, aead::{Aead, Payload}};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
pub const LIMIT: usize = 1024 * 1024;
const DOMAIN: &[u8] = b"MinecraftMacLauncher credential bridge v1";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request { pub op: String, pub key: String, pub value: Option<Vec<u8>> }
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply { pub ok: bool, pub value: Option<Vec<u8>>, pub request_nonce: [u8;12] }
pub fn failure() -> io::Error { io::Error::other("Credential bridge unavailable") }
pub fn permitted(key: &str) -> bool { matches!(key, "dev_license" | "device-tokens" | "user-tokens" | "user-DA") }
pub fn parse_key(s: &str) -> io::Result<[u8;32]> {
 if s.len()!=64 { return Err(failure()); }
 let mut key=[0;32]; for (i,b) in key.iter_mut().enumerate() { *b=u8::from_str_radix(&s[i*2..i*2+2],16).map_err(|_| failure())?; } Ok(key)
}
pub fn send<T: Serialize>(io: &mut impl Write, key: &[u8;32], value: &T, response: bool) -> io::Result<[u8;12]> {
 let plain=serde_json::to_vec(value).map_err(|_|failure())?;
 if plain.len()>LIMIT-64 { return Err(failure()); }
 let mut nonce=[0;12]; rand::thread_rng().fill_bytes(&mut nonce);
 let mut aad=DOMAIN.to_vec(); aad.push(u8::from(response));
 let cipher=ChaCha20Poly1305::new(key.into()).encrypt((&nonce).into(),Payload{msg:&plain,aad:&aad}).map_err(|_|failure())?;
 io.write_all(&((cipher.len()+12) as u32).to_be_bytes())?; io.write_all(&nonce)?; io.write_all(&cipher)?; io.flush()?; Ok(nonce)
}
pub fn receive<T: serde::de::DeserializeOwned>(io: &mut impl Read, key: &[u8;32], response: bool) -> io::Result<(T,[u8;12])> {
 let mut size=[0;4]; io.read_exact(&mut size)?; let size=u32::from_be_bytes(size) as usize;
 if !(28..=LIMIT).contains(&size) { return Err(failure()); }
 let mut bytes=vec![0;size]; io.read_exact(&mut bytes)?;
 let nonce: [u8;12]=bytes[..12].try_into().map_err(|_|failure())?;
 let mut aad=DOMAIN.to_vec(); aad.push(u8::from(response));
 let plain=ChaCha20Poly1305::new(key.into()).decrypt((&nonce).into(),Payload{msg:&bytes[12..],aad:&aad}).map_err(|_|failure())?;
 Ok((serde_json::from_slice(&plain).map_err(|_|failure())?,nonce))
}
pub fn client(op: &str, key_name: &str, value: Option<Vec<u8>>) -> io::Result<Option<Vec<u8>>> {
 if !permitted(key_name) { return Err(failure()); }
 let key=parse_key(&std::env::var("MML_BRIDGE_KEY").map_err(|_|failure())?)?;
 let port: u16=std::env::var("MML_BRIDGE_PORT").map_err(|_|failure())?.parse().map_err(|_|failure())?;
 let addr=std::net::SocketAddr::from(([127,0,0,1],port));
 let mut stream=std::net::TcpStream::connect_timeout(&addr,std::time::Duration::from_secs(5))?;
 stream.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;stream.set_write_timeout(Some(std::time::Duration::from_secs(10)))?;
 let nonce=send(&mut stream,&key,&Request{op:op.into(),key:key_name.into(),value},false)?;
 let (reply,_)=receive::<Reply>(&mut stream,&key,true)?;
 if !reply.ok || reply.request_nonce!=nonce { return Err(failure()); } Ok(reply.value)
}
#[cfg(test)] mod tests {
 use super::*;
 #[test] fn authenticated_and_direction_bound() {
  let mut data=vec![];let key=[7;32];send(&mut data,&key,&Request{op:"get".into(),key:"user-tokens".into(),value:None},false).unwrap();
  assert!(receive::<Request>(&mut &data[..],&[8;32],false).is_err());
  assert!(receive::<Request>(&mut &data[..],&key,true).is_err());
  assert_eq!(receive::<Request>(&mut &data[..],&key,false).unwrap().0.key,"user-tokens");
  let last=data.len()-1;data[last]^=1;assert!(receive::<Request>(&mut &data[..],&key,false).is_err());
 }
 #[test] fn bounds_and_key_allowlist() {
  assert!(receive::<Request>(&mut &[255;4][..],&[0;32],false).is_err());
  assert!(!permitted("arbitrary-account")); assert!(parse_key("bad").is_err());
 }
}
