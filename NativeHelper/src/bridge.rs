use credential_bridge::{Request, Reply, receive, send, failure};
use std::{io::{self, BufRead, Write, Read}, net::TcpListener, collections::HashSet};
/// Only a stdin-provided one-launch secret unlocks the ephemeral localhost listener.
/// stdin EOF is a parent-lifetime signal and ends the broker; no persistent IPC files.
pub fn run() -> io::Result<()> {
 xodus::secrets::init_secrets().map_err(|_|failure())?;
 let mut input=io::stdin().lock();let mut line=String::new();Read::by_ref(&mut input).take(66).read_line(&mut line)?;
 let key=credential_bridge::parse_key(line.trim_end())?;
 drop(input);
 let listener=TcpListener::bind((std::net::Ipv4Addr::LOCALHOST,0))?;
 let port=listener.local_addr()?.port();
 println!("{{\"schema\":1,\"port\":{port}}}");io::stdout().flush()?;
 std::thread::spawn(move || { let mut b=[0;1];use std::io::Read;let _=io::stdin().read(&mut b);std::process::exit(0); });
 let mut seen=HashSet::new();
 for accepted in listener.incoming() {
  let Ok(mut stream)=accepted else {continue};
  stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;stream.set_write_timeout(Some(std::time::Duration::from_secs(5)))?;
  let Ok((request,nonce))=receive::<Request>(&mut stream,&key,false) else {continue};
  if seen.len()>=65536 || !seen.insert(nonce) {continue;}
  let result=(|| -> io::Result<Option<Vec<u8>>> {
   if !credential_bridge::permitted(&request.key) {return Err(failure())}
   let entry=xodus::secrets::get_entry(&request.key).map_err(|_|failure())?;
   match request.op.as_str() {
    "get" if request.value.is_none()=> match entry.get_secret() {Ok(v)=>Ok(Some(v)),Err(keyring_core::Error::NoEntry)=>Ok(None),Err(_)=>Err(failure())},
    "set"=>{let v=request.value.ok_or_else(failure)?;if v.len()>512*1024 {return Err(failure())}entry.set_secret(&v).map_err(|_|failure())?;Ok(None)},
    "remove" if request.value.is_none()=>{match entry.delete_credential(){Ok(())|Err(keyring_core::Error::NoEntry)=>Ok(None),Err(_)=>Err(failure())}},
    _=>Err(failure())
   }
  })();
  let _=send(&mut stream,&key,&Reply{ok:result.is_ok(),value:result.ok().flatten(),request_nonce:nonce},true);
 }
 Ok(())
}
