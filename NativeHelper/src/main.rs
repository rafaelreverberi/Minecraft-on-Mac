//! A bounded stdout schema is the only data crossing into the Swift app.
//! No logger, token serialization, secret file backend, or CLI license dump.
use futures_util::FutureExt;
use serde::Serialize;
use std::panic::AssertUnwindSafe;
use xodus::tokens::TokenManager;
use xodus::models::secrets::Token;
mod license;
mod package;
mod webview;
mod login;

const STORE_ID: &str = "9P5786PJB9RP";
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Reply {
    schema: u8,
    status: &'static str,
    signed_in: bool,
    gamertag: Option<String>,
    entitlement: &'static str,
    available_version: Option<String>,
    error_code: Option<&'static str>,
}
impl Reply {
    fn signed_out() -> Self { Self { schema:1, status:"ok", signed_in:false, gamertag:None, entitlement:"unknown", available_version:None, error_code:None } }
    fn failure(code: &'static str) -> Self { let mut r = Self::signed_out(); r.status="error"; r.error_code=Some(code); r }
}
#[tokio::main]
async fn main() {
    // Upstream panics may include response contents. Never emit their payload.
    std::panic::set_hook(Box::new(|_| {}));
    let command = std::env::args().nth(1).unwrap_or_default();
    let reply = AssertUnwindSafe(run(&command)).catch_unwind().await.unwrap_or_else(|_| Reply::failure("MICROSOFT_SERVICE_FAILED"));
    println!("{}", serde_json::to_string(&reply).unwrap());
}
async fn run(command: &str) -> Reply {
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
        return Reply { schema:1, status:"ok", signed_in:true, gamertag, entitlement:"unknown", available_version:None, error_code:None };
    }
    let content_id = match package::get_content_id(&client, STORE_ID.to_string(), Some("neutral".into())).await {
        Ok(id) => id, Err(_) => return Reply::failure("STORE_LOOKUP_FAILED")
    };
    // Real content license authorization comes BEFORE package lookup. No game/CDN downloads here.
    if license::get_license(&client, &tokens, content_id.clone(), "neutral".into()).await.is_err() {
        return Reply { schema:1, status:"error", signed_in:true, gamertag, entitlement:"unknown", available_version:None, error_code:Some("ENTITLEMENT_NOT_CONFIRMED") };
    }
    let package = match package::get_packages(&client, &tokens, content_id).await {
        Ok(p) => p, Err(_) => return Reply::failure("PACKAGE_LOOKUP_FAILED")
    };
    let version = package.version;
    if version.is_empty() || version.len() > 64 || !version.chars().all(|c| c.is_ascii_digit() || c == '.') { return Reply::failure("PACKAGE_VERSION_INVALID"); }
    Reply { schema:1, status:"ok", signed_in:true, gamertag, entitlement:"verified", available_version:Some(version), error_code:None }
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
        assert_eq!(keys, ["schema", "status", "signedIn", "gamertag", "entitlement", "availableVersion", "errorCode"].into_iter().collect());
    }
}
