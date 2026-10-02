use std::{collections::HashMap, ops::Sub};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserProfileBatch<'t> {
    user_ids: &'t [&'t str],
    settings: &'t [&'t str],
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserProfileSettings {
    id: String,
    value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserProfileEntry {
    id: String,
    settings: Vec<UserProfileSettings>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserProfileBatchResponse {
    profile_users: Vec<UserProfileEntry>,
}

pub struct ProfileUser {
    pub id: String,
    pub settings: std::collections::HashMap<String, String>,
    pub selected: bool,
    pub presense: String,
    pub picture: Option<String>,
    pub gamer_tag: String,
    pub description: String,
}

async fn parse_user_profile_response(r: &UserProfileBatchResponse) -> HashMap<String, ProfileUser> {
    let users = r
        .profile_users
        .iter()
        .map(|user| {
            let mut settings_map = std::collections::HashMap::new();
            for setting in &user.settings {
                settings_map.insert(setting.id.clone(), setting.value.clone());
            }
            let gt = settings_map
                .get("Gamertag")
                .map_or_else(|| String::new(), |f| f.clone());
            (
                user.id.clone(),
                ProfileUser {
                    id: user.id.clone(),
                    selected: false,
                    presense: String::new(),
                    picture: settings_map
                        .get("GameDisplayPicRaw")
                        .map(|f| format!("{}", f)),
                    gamer_tag: settings_map
                        .get("Gamertag")
                        .map_or_else(|| String::new(), |f| f.clone()),
                    settings: settings_map,
                    description: gt,
                },
            )
        })
        .collect::<std::collections::HashMap<_, _>>();

    users
}

pub async fn fetch_user_profiles(
    client: &Client,
    token: &str,
    user_ids: &[&str],
) -> Result<HashMap<String, ProfileUser>, Box<dyn std::error::Error>> {
    let r = client
        .post("https://profile.xboxlive.com/users/batch/profile/settings")
        .header("x-xbl-contract-version", "2")
        .header("Authorization", token)
        .json(&UserProfileBatch {
            user_ids,
            settings: &[
                "AppDisplayName",
                "AppDisplayPicRaw",
                "GameDisplayName",
                "GameDisplayPicRaw",
                "Gamerscore",
                "Gamertag",
                "ModernGamertag",
                "ModernGamertagSuffix",
                "UniqueModernGamertag",
            ],
        })
        .send()
        .await?
        .error_for_status()?
        .json::<UserProfileBatchResponse>()
        .await?;

    Ok(parse_user_profile_response(&r).await)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PeopleHubResponseEntry {
    pub xuid: String,
    pub gamertag: String,
    pub presence_state: String,
    pub display_pic_raw: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PeopleHubResponse {
    people: Vec<PeopleHubResponseEntry>,
}

pub async fn fetch_friends(
    client: &Client,
    token: &str,
) -> Result<HashMap<String, ProfileUser>, Box<dyn std::error::Error>> {
    let r = client
        .get(
            "https://peoplehub.xboxlive.com/users/me/people/friends/decoration/presenceDetail,preferredcolor",
        )
        .header("x-xbl-contract-version", "7")
        .header("Authorization", token)
        .header("Accept-Language", "en-US")// Required for no http 400
        .send()
        .await?
        .error_for_status()?;

    let t: PeopleHubResponse = r.json().await?;

    let mut out = HashMap::new();
    for entry in t.people {
        out.insert(
            entry.xuid.clone(),
            ProfileUser {
                id: entry.xuid,
                selected: false,
                description: format!("{} {}", entry.gamertag, entry.presence_state),
                presense: entry.presence_state,
                picture: Some(entry.display_pic_raw),
                gamer_tag: entry.gamertag,
                settings: HashMap::new(),
            },
        );
    }

    Ok(out)
}

pub async fn fetch_gt(
    client: &Client,
    token: &str,
    gamer_tag: &str,
) -> Result<HashMap<String, ProfileUser>, Box<dyn std::error::Error>> {
    let r = client
        .get(
            format!(
                "https://profile.xboxlive.com/users/gt({})/profile/settings?settings=GameDisplayPicRaw,Gamertag",
                gamer_tag
            ),
        )
        .header("x-xbl-contract-version", "2")
        .header("Authorization", token)
        .send()
        .await?
        .error_for_status()?
        .json::<UserProfileBatchResponse>()
        .await?;

    Ok(parse_user_profile_response(&r).await)
}

pub async fn fetch_gt_tokio(
    client: &Client,
    token: &str,
    gamer_tag: &str,
) -> Option<HashMap<String, ProfileUser>> {
    match fetch_gt(client, token, gamer_tag).await {
        Ok(users) => Some(users),
        Err(e) => {
            eprintln!("Error fetching user profiles: {}", e);
            None
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AchievementReward {
    value: String,
    #[serde(rename = "type")]
    type_: String,
    value_type: String,
    media_asset: Option<AchievementMediaAsset>,
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AchievementMediaAsset {
    url: String,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progression {
    requirements: Vec<ProgressionRequirement>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressionRequirement {
    id: String,
    current: Option<String>,
    target: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementEntry {
    id: String,
    name: String,
    progress_state: String,
    progression: Progression,
    media_assets: Vec<AchievementMediaAsset>,
    description: String,
    rewards: Vec<AchievementReward>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PagingInfo {
    continuation_token: Option<String>,
    total_records: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AchievementsResponse {
    achievements: Vec<AchievementEntry>,
    paging_info: PagingInfo,
}

pub async fn fetch_achivements(
    client: &Client,
    token: &str,
    xuid: &str,
    title_id: i64,
) -> Result<HashMap<String, ProfileUser>, Box<dyn std::error::Error>> {
    let r = client
        .get(
            format!("https://achievements.xboxlive.com/users/xuid({xuid})/achievements?titleId={title_id}&maxItems=1000&includeHidden=true"),
        )
        .header("x-xbl-contract-version", "2")
        .header("Authorization", token)
        .header("Accept-Language", "de-DE")// Required for no http 400
        .send()
        .await?
        .error_for_status()?;

    let t = r.json::<AchievementsResponse>().await?;

    let mut out = HashMap::new();
    for entry in t.achievements {
        out.insert(
            entry.id.clone(),
            ProfileUser {
                id: entry.id,
                selected: false,
                description: format!(
                    "{}\nStatus {}\nReward {}G\n{}",
                    entry.name,
                    entry.progress_state,
                    entry
                        .rewards
                        .iter()
                        .find(|p| p.type_ == "Gamerscore")
                        .map(|p| p.value.clone())
                        .unwrap_or_else(|| "".to_string()),
                    entry.description
                ),
                presense: String::new(),
                picture: entry.media_assets.get(0).map(|f| format!("{}", f.url)),
                gamer_tag: String::new(),
                settings: HashMap::new(),
            },
        );
    }
    Ok(out)
}

pub async fn fetch_achivements_2(
    client: &Client,
    token: &str,
    xuid: &str,
    title_id: i64,
) -> Result<Vec<AchievementEntry>, Box<dyn std::error::Error>> {
    let r = client
        .get(
            format!("https://achievements.xboxlive.com/users/xuid({xuid})/achievements?titleId={title_id}&maxItems=1000&includeHidden=true"),
        )
        .header("x-xbl-contract-version", "2")
        .header("Authorization", token)
        .header("Accept-Language", "en-US")// Required for no http 400
        .send()
        .await?
        .error_for_status()?;

    let t: AchievementsResponse = r.json::<AchievementsResponse>().await?;

    println!(
        "Fetched {} achievements of {}",
        t.achievements.len(),
        t.paging_info.total_records
    );

    Ok(t.achievements)
}
