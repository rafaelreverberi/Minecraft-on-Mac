//! Xodus wire compatibility, using the launcher's own Keychain helper identity.
use std::{sync::Arc, path::Path, io::{self, Write}};
use tokio::net::UnixListener;
use tokio_util::sync::CancellationToken;
use xodus::tokens::TokenManager;
mod connection;
mod simple_context;
const XML_MAGIC: u32 = 0x58445358;
const PROTO_MAGIC: u32 = 0x58445350;
pub async fn run(root: &Path) -> io::Result<()> {
    use std::os::unix::fs::{PermissionsExt, MetadataExt};
    let m = std::fs::symlink_metadata(root)?;
    // Parent creates a new private session directory. Never touch a shared socket.
    if !m.is_dir() || m.mode() & 0o777 != 0o700 || std::fs::canonicalize(root)? != root || !root.file_name().and_then(|s|s.to_str()).is_some_and(|s|s.starts_with("mml-bedrock-")) { return Err(io::Error::other("Service directory invalid")); }
    xodus::secrets::init_secrets().map_err(|_|io::Error::other("Keychain unavailable"))?;
    let tokens=Arc::new(TokenManager::with_keychain_and_memory());
    let client=reqwest::Client::builder().https_only(true).timeout(std::time::Duration::from_secs(60)).build().map_err(|_|io::Error::other("Network unavailable"))?;
    xodus::tokens::device::ensure_device_credentials(&client,&tokens).await;
    let xodus::models::secrets::Token::Legacy(device)=tokens.get_device_sts_token().map_err(|_|io::Error::other("Sign in required"))? else { return Err(io::Error::other("Device invalid")) };
    let path=root.join("xodus.sock");
    let listener=UnixListener::bind(&path)?; // existing socket => fail; never unlink an unknown service
    std::fs::set_permissions(&path,std::fs::Permissions::from_mode(0o600))?;
    println!("{{\"schema\":1,\"ready\":true}}");io::stdout().flush()?;
    let cleanup_root=root.to_path_buf();let cleanup_socket=path.clone();let original_inode=m.ino();
    std::thread::spawn(move || {
        use std::io::Read;
        let mut byte=[0];let _=std::io::stdin().read(&mut byte);
        if std::fs::symlink_metadata(&cleanup_root).is_ok_and(|m|m.ino()==original_inode) {
            let _=std::fs::remove_file(cleanup_socket);let _=std::fs::remove_dir(cleanup_root);
        }
        std::process::exit(0);
    });
    let cancel=CancellationToken::new();
    // No tracing subscriber is installed. Sensitive upstream trace records are discarded.
    loop {
        let (stream,_)=listener.accept().await?;
        let credentials=stream.peer_cred()?;
        if credentials.uid()!=m.uid() { continue; }
        let child=cancel.child_token(); let device=device.clone(); let tokens=tokens.clone();
        tokio::spawn(async move { let _=tokio::time::timeout(std::time::Duration::from_secs(900),connection::router::route(stream,child,device,tokens)).await; });
    }
}
