
use xodus::XBOX_LIVE_PACKAGES_PC;
use xodus::api::displaycatalog::find_products_by_id;
use xodus::models::packagespc::{PackageDetails, PackageResponse};
use xodus::models::secrets::Token;
use xodus::tokens::TokenManager;

pub async fn get_content_id(
    client: &reqwest::Client,
    product: String,
    market: Option<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let displaycatalog = find_products_by_id(
        client,
        product,
        market.clone().unwrap_or("neutral".to_owned()),
        vec!["en".to_string(), "neutral".to_string()],
    )
    .await?;

    let product_details = displaycatalog.product;

    let mut found_package = None;
    let mut subprods: Vec<String> = vec![];
    'o: for availability in &product_details.display_sku_availabilities {
        for package in &availability.sku.properties.packages {
            if package
                .platform_dependencies
                .iter()
                .any(|dep| dep.platform_name == "Windows.Desktop")
            {
                found_package = Some(package);
                break 'o;
            }
        }
        for availability in &availability.availabilities {
            if let Some(licensing_data) = &availability.licensing_data {
                for satisfies in &licensing_data.satisfying_entitlement_keys {
                    for entitlement_key in &satisfies.entitlement_keys {
                        let key: Vec<&str> = entitlement_key.split(":").collect();
                        if key.len() == 3 && key[0] == "big" {
                            subprods.push(key[1].to_string());
                        }
                    }
                }
            }
        }
    }
    subprods.sort();
    subprods.dedup();

    let Some(package) = found_package else {

        return Err(Box::new(std::io::Error::other(
            "Windows.Desktop package not found, if you believe this is an error, please report it",
        )));
    };

    let Some(content_id) = &package.content_id else {

        return Err(Box::new(std::io::Error::other(
            "ContentId not found, if you believe this is an error, please report it",
        )));
    };
    Ok(content_id.to_owned())
}

pub async fn get_packages(
    client: &reqwest::Client,
    tokens: &TokenManager,
    content_id: String,
) -> Result<PackageDetails, Box<dyn std::error::Error>> {
    let dev_token = tokens.get_device_sts_token().unwrap();
    let Token::Legacy(dev_token) = dev_token else {
        return Err(Box::new(std::io::Error::other("Invalid STS token")));
    };
    let user_token = tokens.get_user_sts_token().unwrap();
    let Token::Legacy(legacy) = user_token else {
        return Err(Box::new(std::io::Error::other("Unsupported user token")));
    };

    let xsts_token =
        xodus::api::xbox::run(client, dev_token, legacy, "http://update.xboxlive.com").await;

    let response = client
        .get(format!(
            "{XBOX_LIVE_PACKAGES_PC}/GetBasePackage/{content_id}"
        ))
        .header("x-xbl-contract-version", "3")
        .header(
            "Authorization",
            xodus::api::xbox::get_xsts_auth_header(xsts_token),
        )
        .send()
        .await
        .unwrap();

    let res: PackageResponse = response.json().await.expect("Failed to get data");

    let PackageResponse::Found(package) = res else {
        return Err(Box::new(std::io::Error::other(
            "Package was not found, is it owned by the user?",
        )));
    };
    Ok(package)
}

/// Xbox Packages PC uses `major.minor.build.revision.package-GUID`, not just four numbers.
pub fn normalize_version(raw: &str) -> Result<(String, Option<String>), std::io::Error> {
    let parts: Vec<_> = raw.splitn(5, '.').collect();
    if !(4..=5).contains(&parts.len()) { return Err(std::io::Error::other("Package version invalid")); }
    let numbers: Result<Vec<u16>,_> = parts[..4].iter().map(|p| p.parse()).collect();
    let numbers = numbers.map_err(|_|std::io::Error::other("Package version invalid"))?;
    let revision = if parts.len()==5 { Some(uuid::Uuid::parse_str(parts[4]).map_err(|_|std::io::Error::other("Package revision invalid"))?.to_string()) } else { None };
    Ok((numbers.iter().map(u16::to_string).collect::<Vec<_>>().join("."), revision))
}
#[cfg(test)] mod version_tests {
    use super::*;
    #[test] fn xbox_composite_version_is_not_confused_with_build_number() {
        let (version,revision)=normalize_version("1.1.1.0.55640c99-2b99-4abd-ba3b-ed4198e427d3").unwrap();
        assert_eq!(version,"1.1.1.0");assert_eq!(revision.as_deref(),Some("55640c99-2b99-4abd-ba3b-ed4198e427d3"));
        assert!(normalize_version("1.1.1.0.untrusted").is_err());assert!(normalize_version("1.1.1").is_err());
    }
}

/// Developer inspection: only digest metadata and host/status are emitted. Never URLs,
/// account identifiers, keys, authorization headers or raw service errors.
#[cfg(feature="bootstrap-pins")]
pub async fn audit_bedrock(header_out: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    xodus::secrets::init_secrets()?;
    let tokens = TokenManager::with_keychain_and_memory();
    if tokens.get_user().is_err() { return Err(std::io::Error::other("SIGN_IN_REQUIRED").into()); }
    let client = reqwest::Client::builder().https_only(true).timeout(std::time::Duration::from_secs(20)).user_agent("MinecraftMacLauncher/0.2").build()?;
    xodus::tokens::device::ensure_device_credentials(&client, &tokens).await;
    let id = get_content_id(&client, crate::game::Game::Bedrock.store_id().into(), Some("neutral".into())).await?;
    crate::license::get_license(&client, &tokens, id.clone(), "neutral".into()).await?;
    let package = get_packages(&client, &tokens, id.clone()).await?;
    let Token::Legacy(device) = tokens.get_device_sts_token()? else { return Err(std::io::Error::other("DEVICE_AUTH_REQUIRED").into()) };
    let Token::Legacy(user) = tokens.get_user_sts_token()? else { return Err(std::io::Error::other("SIGN_IN_REQUIRED").into()) };
    let xbox = xodus::api::xbox::run(&client, device, user, "http://update.xboxlive.com").await;
    let raw: serde_json::Value = client.get(format!("{XBOX_LIVE_PACKAGES_PC}/GetBasePackage/{id}"))
        .header("x-xbl-contract-version", "3").header("Authorization", xodus::api::xbox::get_xsts_auth_header(xbox))
        .send().await?.error_for_status()?.json().await?;
    fn shape(value: &serde_json::Value, depth: usize) -> serde_json::Value {
        if depth > 5 {return serde_json::json!("bounded");}
        match value {
            serde_json::Value::Object(map) => serde_json::Value::Object(map.iter().take(100).map(|(k,v)|(k.clone(),shape(v, depth+1))).collect()),
            serde_json::Value::Array(items) => serde_json::json!({"count":items.len(),"item":items.first().map(|v|shape(v,depth+1))}),
            serde_json::Value::String(text) => serde_json::json!({"stringLength":text.len()}),
            serde_json::Value::Number(_) => serde_json::json!("number"),
            serde_json::Value::Bool(_) => serde_json::json!("boolean"),
            serde_json::Value::Null => serde_json::Value::Null,
        }
    }
    println!("{}",serde_json::json!({"schema":1,"metadataShape":shape(&raw,0)}));
    fn digest(value: &str) -> Option<&str> {
        use base64::Engine;
        (([40,64].contains(&value.len()) && value.bytes().all(|b|b.is_ascii_hexdigit())) || base64::engine::general_purpose::STANDARD.decode(value).is_ok_and(|b| [20,32].contains(&b.len()))).then_some(value)
    }
    println!("{}", serde_json::json!({"schema":1,"version":normalize_version(&package.version)?.0,"hashOfHashes":package.hash_of_hashes.as_deref().and_then(digest)}));
    for file in package.package_files.iter().filter(|f|f.file_name.to_ascii_lowercase().ends_with(".msixvc")) {
        println!("{}", serde_json::json!({"schema":1,"size":file.file_size,"fileHash":digest(&file.file_hash)}));
        if let Some(path) = header_out {
            use std::io::Write;
            let url=reqwest::Url::parse(&format!("{}{}",file.cdn_root_paths.first().ok_or_else(||std::io::Error::other("SOURCE_MISSING"))?,file.relative_url))?;
            if !matches!(url.host_str(),Some("assets1.xboxlive.com"|"assets2.xboxlive.com")) || url.username()!="" || url.password().is_some() || url.query().is_some() || url.fragment().is_some() {return Err(std::io::Error::other("SOURCE_INVALID").into());}
            let public=reqwest::Client::builder().timeout(std::time::Duration::from_secs(30)).build()?;
            let response=public.get(url).header("Range","bytes=0-4095").send().await?.error_for_status()?;
            if response.status()!=reqwest::StatusCode::PARTIAL_CONTENT || response.content_length()!=Some(4096) {return Err(std::io::Error::other("RANGE_INVALID").into());}
            let bytes=response.bytes().await?;if bytes.len()!=4096{return Err(std::io::Error::other("RANGE_INVALID").into());}
            let mut output=std::fs::OpenOptions::new().write(true).create_new(true).open(path)?;output.write_all(&bytes)?;output.sync_all()?;
        }

        let alternate_roots = ["https://dlassets.xboxlive.com/", "https://dlassets2.xboxlive.com/", "https://assets1.xboxlive.com.delivery.microsoft.com/", "https://assets2.xboxlive.com.delivery.microsoft.com/"];
        for root in file.cdn_root_paths.iter().chain(file.background_cdn_root_paths.iter()).map(String::as_str).chain(alternate_roots).take(12) {
            let Ok(mut url)=reqwest::Url::parse(&format!("{}{}",root,file.relative_url)) else {continue};
            let Some(host)=url.host_str().map(str::to_owned) else {continue};
            if !(host.ends_with(".xboxlive.com") || host.ends_with(".xboxlive.com.delivery.microsoft.com")) || url.username()!="" || url.password().is_some() { continue; }
            if url.set_scheme("https").is_err(){continue;}
            let result=client.head(url).send().await;
            println!("{}",serde_json::json!({"schema":1,"host":host,"verifiedHttps":result.as_ref().is_ok_and(|r|r.status().is_success()),"status":result.as_ref().ok().map(|r|r.status().as_u16())}));
        }
    }
    Ok(())
}

/// Engineering download of an encrypted container for independent signature/hash-tree
/// review. This is unavailable in release builds and cannot install or decrypt a game.
#[cfg(feature="bootstrap-pins")]
pub async fn review_download(output:&std::path::Path)->Result<(),Box<dyn std::error::Error>> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;
    use sha2::{Digest,Sha256};
    if output.exists() {return Err(std::io::Error::other("OUTPUT_EXISTS").into());}
    xodus::secrets::init_secrets()?;let tokens=TokenManager::with_keychain_and_memory();
    let client=reqwest::Client::builder().https_only(true).timeout(std::time::Duration::from_secs(60)).build()?;
    xodus::tokens::device::ensure_device_credentials(&client,&tokens).await;
    let id=get_content_id(&client,crate::game::Game::Bedrock.store_id().into(),Some("neutral".into())).await?;
    crate::license::get_license(&client,&tokens,id.clone(),"neutral".into()).await?;
    let package=get_packages(&client,&tokens,id.clone()).await?;
    let (version,revision)=normalize_version(&package.version)?;crate::game::Game::Bedrock.validate(&version)?;
    let files:Vec<_>=package.package_files.iter().filter(|f|f.file_name.to_ascii_lowercase().ends_with(".msixvc")).collect();
    if files.len()!=1{return Err(std::io::Error::other("PACKAGE_INVALID").into());}
    let file=files[0];if file.file_size!=2069975040 {return Err(std::io::Error::other("PACKAGE_CHANGED").into());}
    let url=reqwest::Url::parse(&format!("{}{}",file.cdn_root_paths.first().ok_or_else(||std::io::Error::other("SOURCE_MISSING"))?,file.relative_url))?;
    if !matches!(url.host_str(),Some("assets1.xboxlive.com"|"assets2.xboxlive.com")) || url.scheme()!="http" || url.username()!="" || url.password().is_some() || url.query().is_some() || url.fragment().is_some(){return Err(std::io::Error::other("SOURCE_INVALID").into());}
    let public=reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).connect_timeout(std::time::Duration::from_secs(30)).read_timeout(std::time::Duration::from_secs(90)).build()?;
    let response=public.get(url).send().await?.error_for_status()?;
    if response.status()!=reqwest::StatusCode::OK || response.content_length()!=Some(file.file_size as u64){return Err(std::io::Error::other("SIZE_INVALID").into());}
    let mut options=std::fs::OpenOptions::new();options.write(true).create_new(true);
    #[cfg(unix)] {use std::os::unix::fs::OpenOptionsExt;options.mode(0o600);}
    let mut output=tokio::fs::File::from_std(options.open(output)?);let mut stream=response.bytes_stream();let mut done=0u64;let mut hash=Sha256::new();let mut clock=std::time::Instant::now();
    while let Some(bytes)=stream.next().await {let bytes=bytes?;done=done.checked_add(bytes.len() as u64).ok_or_else(||std::io::Error::other("SIZE_INVALID"))?;if done>file.file_size as u64{return Err(std::io::Error::other("SIZE_INVALID").into());}hash.update(&bytes);output.write_all(&bytes).await?;if clock.elapsed().as_secs()>=5 {println!("{}",serde_json::json!({"schema":1,"reviewBytes":done,"total":file.file_size}));clock=std::time::Instant::now();}}
    output.sync_all().await?;if done!=file.file_size as u64{return Err(std::io::Error::other("SIZE_INVALID").into());}
    let sha256:String=hash.finalize().iter().map(|b|format!("{b:02x}")).collect();println!("{}",serde_json::json!({"schema":1,"contentId":id,"revision":revision,"size":done,"sha256":sha256,"verified":false}));Ok(())
}
