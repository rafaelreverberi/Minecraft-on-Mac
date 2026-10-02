//! A bounded stdout schema is the only data crossing into the Swift app.
//! No logger, token serialization, secret file backend, or CLI license dump.
use futures_util::FutureExt;
use serde::Serialize;
use std::panic::AssertUnwindSafe;
use xodus::tokens::TokenManager;
use xodus::models::secrets::Token;
mod acquire;
mod bridge;
mod license;
mod package;
mod webview;
mod login;

mod game;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Reply {
    schema: u8,
    status: &'static str,
    signed_in: bool,
    gamertag: Option<String>,
    entitlement: &'static str,
    available_version: Option<String>,
    available_revision: Option<String>,
    error_code: Option<&'static str>,
}
impl Reply {
    fn signed_out() -> Self { Self { schema:1, status:"ok", signed_in:false, gamertag:None, entitlement:"unknown", available_version:None, available_revision:None, error_code:None } }
    fn failure(code: &'static str) -> Self { let mut r = Self::signed_out(); r.status="error"; r.error_code=Some(code); r }
}
#[tokio::main]
async fn main() {
    // Upstream panics may include response contents. Never emit their payload.
    std::panic::set_hook(Box::new(|_| {}));
    let command = std::env::args().nth(1).unwrap_or_default();
    if command == "install" {
        let stage=std::env::args().nth(2).unwrap_or_default();
        let game = match game::Game::parse(&std::env::args().nth(3).unwrap_or_default()) { Ok(g)=>g,Err(_)=>{println!("{{\"schema\":1,\"type\":\"error\",\"code\":\"INSTALL_FAILED\"}}");std::process::exit(1)} };
        let result=AssertUnwindSafe(acquire::install(std::path::Path::new(&stage), game)).catch_unwind().await;
        if !matches!(result,Ok(Ok(()))) {
            let code=match result { Ok(Err(e))=>match e.to_string().as_str() {
                "BEDROCK_VERSION_TOO_OLD"=>"BEDROCK_VERSION_TOO_OLD",
                "BEDROCK_COMPATIBILITY_UNVERIFIED"=>"BEDROCK_COMPATIBILITY_UNVERIFIED",
                "BEDROCK_PACKAGE_INTEGRITY_UNVERIFIED"=>"BEDROCK_PACKAGE_INTEGRITY_UNVERIFIED",
                _=>"INSTALL_FAILED"
            },_=>"INSTALL_FAILED" };
            println!("{}",serde_json::json!({"schema":1,"type":"error","code":code}));std::process::exit(1);
        } return;
    }
    #[cfg(feature="bootstrap-pins")]
    if command == "bedrock-review-download" {
        let out = std::env::args().nth(2).unwrap_or_default();
        let result = AssertUnwindSafe(package::review_download(std::path::Path::new(&out))).catch_unwind().await;
        if !matches!(result, Ok(Ok(()))) { println!("{{\"schema\":1,\"code\":\"PACKAGE_REVIEW_FAILED\"}}"); std::process::exit(1); }
        return;
    }
    #[cfg(feature="bootstrap-pins")]
    if command == "package-audit" {
        let result = AssertUnwindSafe(package::audit_bedrock(std::env::args().nth(2).as_deref())).catch_unwind().await;
        if !matches!(result, Ok(Ok(()))) { println!("{{\"schema\":1,\"code\":\"PACKAGE_AUDIT_FAILED\"}}"); std::process::exit(1); }
        return;
    }
    if command == "bedrock-service" {
        let root = std::env::args().nth(2).unwrap_or_default();
        let _ = AssertUnwindSafe(bedrock_account_service::run(std::path::Path::new(&root))).catch_unwind().await;
        return;
    }
    #[cfg(feature="bootstrap-pins")]
    if command == "sdk-public-audit" {
        let args:Vec<_>=std::env::args().collect();
        if args.len()!=4 || acquire::extract_public_signing_keys(std::path::Path::new(&args[2]),std::path::Path::new(&args[3])).is_err(){std::process::exit(1)};return;
    }
    if command == "sdk-headers" {
        let args:Vec<_>=std::env::args().collect();
        if args.len()!=4 || acquire::extract_headers(std::path::Path::new(&args[2]),std::path::Path::new(&args[3])).is_err(){std::process::exit(1)}; return;
    }
    if command == "sdk-test" {
        let args:Vec<_>=std::env::args().collect();
        if args.len()!=4 || acquire::extract_sdk(std::path::Path::new(&args[2]),std::path::Path::new(&args[3])).is_err(){std::process::exit(1)}; return;
    }
    if command == "bridge" { let _ = bridge::run(); return; }
    let reply = AssertUnwindSafe(run(&command, &std::env::args().nth(2).unwrap_or_else(||"dungeons2".into()))).catch_unwind().await.unwrap_or_else(|_| Reply::failure("MICROSOFT_SERVICE_FAILED"));
    println!("{}", serde_json::to_string(&reply).unwrap());
}
async fn run(command: &str, game_id: &str) -> Reply {
    let game = match game::Game::parse(game_id) { Ok(g)=>g,Err(_)=>return Reply::failure("COMMAND_INVALID") };
    if !matches!(command, "status" | "login" | "logout" | "check") { return Reply::failure("COMMAND_INVALID"); }
    if xodus::secrets::init_secrets().is_err() { return Reply::failure("KEYCHAIN_UNAVAILABLE"); }
    let tokens = TokenManager::with_keychain_and_memory();
    if command == "logout" {
        // Only this helper's private namespace. Never the reference game account.
        for key in ["dev_license", "device-tokens", "user-tokens", "user-DA"] {
            if let Ok(entry) = xodus::secrets::get_entry(key) {
                if entry.get_secret().is_ok() && entry.delete_credential().is_err() { return Reply::failure("SIGN_OUT_FAILED"); }
            } else { return Reply::failure("SIGN_OUT_FAILED"); }
        }
        return Reply::signed_out();
    }
    if command == "status" {
        let mut r = Reply::signed_out(); r.signed_in = tokens.get_user().is_ok() && tokens.get_user_sts_token().is_ok();
        return r;
    }
    let client = match reqwest::Client::builder().https_only(true).timeout(std::time::Duration::from_secs(60)).user_agent("MinecraftMacLauncher/0.1").build() {
        Ok(c) => c, Err(_) => return Reply::failure("NETWORK_UNAVAILABLE")
    };
    xodus::tokens::device::ensure_device_credentials(&client, &tokens).await;
    if command == "login" {
        let _ = login::run(&client, &tokens).await;
        if tokens.get_user().is_err() || tokens.get_user_sts_token().is_err() { return Reply::failure("LOGIN_CANCELLED"); }
    }
    let Ok(Token::Legacy(user)) = tokens.get_user_sts_token() else { return Reply::failure("SIGN_IN_REQUIRED"); };
    let Ok(Token::Legacy(device)) = tokens.get_device_sts_token() else { return Reply::failure("DEVICE_AUTH_REQUIRED"); };
    let xbox = xodus::api::xbox::run(&client, device, user, "http://xboxlive.com").await;
    let gamertag = xbox.gamertag().filter(|s| s.len() <= 256 && !s.chars().any(char::is_control)).map(str::to_string);
    if command == "login" {
        return Reply { schema:1, status:"ok", signed_in:true, gamertag, entitlement:"unknown", available_version:None, available_revision:None, error_code:None };
    }
    let content_id = match package::get_content_id(&client, game.store_id().to_string(), Some("neutral".into())).await {
        Ok(id) => id, Err(_) => return Reply::failure("STORE_LOOKUP_FAILED")
    };
    // Real content license authorization comes BEFORE package lookup. No game/CDN downloads here.
    if license::get_license(&client, &tokens, content_id.clone(), "neutral".into()).await.is_err() {
        return Reply { schema:1, status:"error", signed_in:true, gamertag, entitlement:"unknown", available_version:None, available_revision:None, error_code:Some("ENTITLEMENT_NOT_CONFIRMED") };
    }
    let package = match package::get_packages(&client, &tokens, content_id.clone()).await {
        Ok(p) => p, Err(_) => return Reply::failure("PACKAGE_LOOKUP_FAILED")
    };
    let (version, revision) = match package::normalize_version(&package.version) { Ok(v) => v, Err(_) => return Reply::failure("PACKAGE_VERSION_INVALID") };
    Reply { schema:1, status:"ok", signed_in:true, gamertag, entitlement:"verified", available_version:Some(version), available_revision:revision, error_code:None }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failure_is_unknown_not_a_license() {
        let reply = Reply::failure("ENTITLEMENT_NOT_CONFIRMED");
        assert_eq!(reply.entitlement, "unknown");
        assert!(reply.available_version.is_none());
    }
    #[test]
    fn reply_has_only_allowlisted_fields() {
        let json = serde_json::to_value(Reply::signed_out()).unwrap();
        let keys: std::collections::BTreeSet<_> = json.as_object().unwrap().keys().map(String::as_str).collect();
        assert_eq!(keys, ["schema", "status", "signedIn", "gamertag", "entitlement", "availableVersion", "availableRevision", "errorCode"].into_iter().collect());
    }
}
