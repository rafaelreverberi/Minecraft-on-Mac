
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
