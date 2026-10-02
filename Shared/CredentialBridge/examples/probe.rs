//! Synthetic cross-platform IPC acceptance, never reads or writes a Keychain entry.
use credential_bridge::*;
fn main()->std::io::Result<()> {
 if matches!(std::env::args().nth(1).as_deref(),Some("client"|"client-empty")) {
  let expected=if std::env::args().nth(1).as_deref()==Some("client-empty"){None}else{Some(b"synthetic-fixture-only".to_vec())};
  if client("get","user-tokens",None)?!=expected {return Err(failure())}
  return Ok(())
 }
 let listener=std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST,0))?;
 let key=[19u8;32];let port=listener.local_addr()?.port();
 let exe=std::env::args().nth(1).ok_or_else(failure)?;let wine=std::env::args().nth(2).ok_or_else(failure)?;
 let bottle=std::env::args().nth(3).ok_or_else(failure)?;
 let server=std::thread::spawn(move||->std::io::Result<()>{
  let (mut stream,_)=listener.accept()?;stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
  let (request,nonce)=receive::<Request>(&mut stream,&key,false)?;
  if request.op!="get"||request.key!="user-tokens"||request.value.is_some(){return Err(failure())}
  send(&mut stream,&key,&Reply{ok:true,value:Some(b"synthetic-fixture-only".to_vec()),request_nonce:nonce},true)?;Ok(())
 });
 let status=std::process::Command::new(wine).args(["--debugmsg","-all","--bottle",&bottle,&exe,"client"])
  .env("MML_BRIDGE_KEY", "13".repeat(32)).env("MML_BRIDGE_PORT",port.to_string())
  .env("WINEDEBUG","-all").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status()?;
 if !status.success(){return Err(failure())}server.join().map_err(|_|failure())??;println!("Synthetic macOS to Win64 credential bridge passed");Ok(())
}
