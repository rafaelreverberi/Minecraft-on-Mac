//! Real licensed package acquisition. Local transactions never touch a selected version.
use std::{path::{Path,PathBuf}, io::{Read,Write}, collections::{HashMap,HashSet}};
use tokio::io::{AsyncWriteExt};
use futures_util::StreamExt;
use sha2::{Sha256,Digest};
use base64::Engine;
use xodus::tokens::TokenManager;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const SDK_URL:&str="https://github.com/microsoft/GDK/releases/download/April-2025-Update-6-v2504.6.4142/GDK_2504.6.4142.zip";
const SDK_HASH:&str="b10d02d2b3619129fb5d176a762fb10961dd4a59c3a19dc64a6e38e03deb590f";
const NUGET_URL:&str="https://api.nuget.org/v3-flatcontainer/microsoft.gdk.windows/2604.4.7897/microsoft.gdk.windows.2604.4.7897.nupkg";
const COMPONENTS:[(&str,&str);3]=[
 ("XCurl2504.dll","cd41c0a777861bee4eaf36e6cd8a26ffadaf78a5e4b0f7eb47d34768a3091d80"),
 ("xgameruntime.gdk.dll","815d0c5b0aa5c84eb6104168da551a4922f49f8dd02dbdf3bbc5119beec11b59"),
 ("libHttpClient.GDK.dll","ca94296a372096153806422f5236144c695dfa9cd77607ef1761afc0e5a184b1")];
fn err() -> Box<dyn std::error::Error> {std::io::Error::other("Acquisition validation failed").into()}
fn hex(bytes:impl AsRef<[u8]>) -> String {bytes.as_ref().iter().map(|b|format!("{b:02x}")).collect()}
fn progress(phase:&str,completed:u64,total:u64) {
 println!("{}",serde_json::json!({"schema":1,"type":"progress","phase":phase,"completed":completed,"total":total}));
}
fn safe_relative(name:&str)->Result<PathBuf> {
 let normalized=name.replace('\\',"/");
 if normalized.is_empty()||normalized.starts_with('/')||normalized.contains(':')||normalized.chars().any(char::is_control) {return Err(err())}
 if normalized.split('/').any(|s|s.is_empty()||s=="."||s=="..") {return Err(err())}
 Ok(PathBuf::from(normalized))
}
fn new_dir(path:&Path)->Result<()> {
 std::fs::create_dir(path)?;
 #[cfg(unix)] {use std::os::unix::fs::PermissionsExt;std::fs::set_permissions(path,std::fs::Permissions::from_mode(0o700))?;}
 Ok(())
}
fn create(path:&Path)->Result<std::fs::File>{
 let mut o=std::fs::OpenOptions::new();o.write(true).create_new(true);
 #[cfg(unix)] {use std::os::unix::fs::OpenOptionsExt;o.mode(0o600);}
 Ok(o.open(path)?)
}
fn hash(path:&Path)->Result<String>{let mut f=std::fs::File::open(path)?;let mut h=Sha256::new();let mut b=[0;1024*1024];loop{let n=f.read(&mut b)?;if n==0{break}h.update(&b[..n]);}Ok(hex(h.finalize()))}
fn trusted_component(out:&Path,name:&str,data:&[u8])->Result<bool>{
 let Some((_,expected))=COMPONENTS.iter().find(|(n,_)|*n==name) else {return Err(err())};
 if hex(Sha256::digest(data))!=*expected {return Ok(false)}
 if !out.join(name).exists(){create(&out.join(name))?.write_all(data)?;} Ok(true)
}
async fn download(client:&reqwest::Client,url:&str,path:&Path,size:Option<u64>,limit:u64,expected:Option<&str>,phase:&str)->Result<String> {
 let mut url=reqwest::Url::parse(url)?;
 let legacy_public_package=phase=="downloading-game" && url.scheme()=="http" && url.query().is_none() && url.fragment().is_none() && matches!(url.host_str(),Some("assets1.xboxlive.com"|"assets2.xboxlive.com")) && (expected.is_some() || cfg!(feature="bootstrap-pins"));
 // Microsoft's legacy CDN offers encrypted public containers over HTTP. No account credentials
 // are sent there; release acquisition requires a pinned full SHA-256 before any decryption.
 // HTTPS verification stays enabled for all HTTPS requests and every account/SDK endpoint.
 if url.scheme()=="http" && !legacy_public_package { url.set_scheme("https").map_err(|_|err())?; }
 if (!legacy_public_package && url.scheme()!="https") || url.username()!="" || url.password().is_some(){return Err(err())}
 let response=client.get(url).send().await?.error_for_status()?;
 if size.is_some_and(|n|response.content_length().is_some_and(|len|len!=n)) {return Err(err())}
 if response.content_length().is_some_and(|n|n>limit){return Err(err())}
 let file=create(path)?;let mut file=tokio::fs::File::from_std(file);
 let mut stream=response.bytes_stream();let mut done=0u64;let mut sha256=Sha256::new();let mut sha1=sha1::Sha1::new();let mut clock=std::time::Instant::now();
 while let Some(chunk)=stream.next().await {
  let data=chunk?;done=done.checked_add(data.len() as u64).ok_or_else(err)?;
  if done>limit||size.is_some_and(|n|done>n){return Err(err())}
  sha256.update(&data);sha1.update(&data);file.write_all(&data).await?;
  if clock.elapsed().as_millis()>250 {progress(phase,done,size.unwrap_or(0));clock=std::time::Instant::now();}
 }
 file.sync_all().await?;
 if done==0||size.is_some_and(|n|done!=n){return Err(err())}
 let a=sha256.finalize();let b=sha1.finalize();
 if let Some(expected)=expected {
  let valid=hex(a)==expected.to_lowercase()||hex(b)==expected.to_lowercase()||base64::engine::general_purpose::STANDARD.encode(a)==expected||base64::engine::general_purpose::STANDARD.encode(b)==expected;
  if !valid {return Err(err())}
 }
 progress(phase,done,done);Ok(hex(a))
}
/// Zip, MSI and CAB parsers ship inside the signed helper. No shell or developer tools.
pub fn extract_sdk(archive:&Path,out:&Path)->Result<()> {
 if hash(archive)?!=SDK_HASH{return Err(err())}
 let mut z=zip::ZipArchive::new(std::fs::File::open(archive)?)?;
 let mut cabinets=vec![];let mut ids:HashMap<String,String>=HashMap::new();
 for i in 0..z.len(){
  let mut entry=z.by_index(i)?;let name=entry.name().to_string();
  if !name.contains("Installers/"){continue;}
  if name.ends_with(".msi") {
   if entry.size()>64*1024*1024{return Err(err())}let mut data=vec![];entry.read_to_end(&mut data)?;
   let mut m=msi::Package::open(std::io::Cursor::new(data))?;
   if !m.has_table("File"){continue}
   for row in m.select_rows(msi::Select::table("File"))? {
    let Some(n)=row["FileName"].as_str() else {continue};let long=n.split('|').next_back().unwrap_or(n);
    let target=match long.to_lowercase().as_str(){"xcurl.dll"=>Some("XCurl2504.dll"),"libhttpclient.gdk.dll"=>Some("libHttpClient.GDK.dll"),_=>None};
    if let (Some(target),Some(id))=(target,row["File"].as_str()){ids.insert(id.into(),target.into());}
   }
  }else if name.ends_with(".cab") {cabinets.push(name);}
  else if name.ends_with("Microsoft.VCLibs.x64.14.00.appx") {
   if entry.size()>32*1024*1024 {return Err(err())}let mut b=vec![];entry.read_to_end(&mut b)?;
   let mut vc=zip::ZipArchive::new(std::io::Cursor::new(b))?;
   let vcdir=out.join("VC");if !vcdir.exists(){new_dir(&vcdir)?;}
   for j in 0..vc.len(){let mut f=vc.by_index(j)?;let n=f.name().to_ascii_lowercase();
    if !n.contains('/')&&n.ends_with(".dll")&&f.size()<16*1024*1024 {let mut data=vec![];f.read_to_end(&mut data)?;create(&vcdir.join(&n))?.write_all(&data)?;}
   }
  }
 }
 for name in cabinets {
  let mut entry=z.by_name(&name)?;if entry.size()>600*1024*1024{return Err(err())}
  let mut bytes=vec![];entry.read_to_end(&mut bytes)?;let mut c=cab::Cabinet::new(std::io::Cursor::new(bytes))?;
  let targets:Vec<(String,String)>=c.folder_entries().flat_map(|folder|folder.file_entries()).filter_map(|file|{
   if file.uncompressed_size()>32*1024*1024{return None}
   ids.get(file.name()).map(|target|(file.name().to_string(),target.clone()))
  }).collect();
  for (id,target) in targets{let mut data=vec![];c.read_file(&id)?.read_to_end(&mut data)?;trusted_component(out,&target,&data)?;}
 }
 for n in ["XCurl2504.dll"] {if !out.join(n).exists(){return Err(err())}}

 Ok(())
}
fn extract_nuget(archive:&Path,out:&Path)->Result<()> {
 let mut z=zip::ZipArchive::new(std::fs::File::open(archive)?)?;
 let matches:Vec<String>=(0..z.len()).filter_map(|i|{let f=z.by_index(i).ok()?;f.name().ends_with("native/260404/windows/bin/x64/xgameruntime.dll").then(||f.name().to_string())}).collect();
 if matches.len()!=1{return Err(err())}let mut f=z.by_name(&matches[0])?;
 if f.size()>32*1024*1024{return Err(err())}let mut data=vec![];f.read_to_end(&mut data)?;
 if !trusted_component(out,"xgameruntime.gdk.dll",&data)?{return Err(err())}
 drop(f);
 let mut notice=z.by_name("LICENSE.md")?;if notice.size()>1024*1024{return Err(err())}
 let mut text=vec![];notice.read_to_end(&mut text)?;create(&out.join("Microsoft-GDK-LICENSE.md"))?.write_all(&text)?;Ok(())
}
pub async fn install(stage:&Path,game_definition:crate::game::Game)->Result<()> {
 // Parent supplied a UUID staging directory. Never accept arbitrary pre-existing game trees.
 if !stage.is_dir()||!stage.file_name().and_then(|n|n.to_str()).is_some_and(|n|n.starts_with(".staging-")&&uuid::Uuid::parse_str(&n[9..]).is_ok())||stage.join("Game").exists(){return Err(err())}
 let canonical=std::fs::canonicalize(stage)?;if canonical!=stage{return Err(err())}
 let receipt:serde_json::Value=serde_json::from_reader(std::fs::File::open(stage.join("metadata.json"))?)?;
 let stage_id=&stage.file_name().and_then(|s|s.to_str()).ok_or_else(err)?[9..];
 if receipt["id"].as_str()!=Some(stage_id)||receipt["gameId"].as_str()!=Some(game_definition.id())||receipt["storeId"].as_str()!=Some(game_definition.store_id())||receipt["managed"]!=true||receipt["path"].as_str().and_then(|s|reqwest::Url::parse(s).ok()).and_then(|u|u.to_file_path().ok())!=Some(stage.join("Game")) {return Err(err())}
 let versions=stage.parent().ok_or_else(err)?;
 if versions.file_name().and_then(|s|s.to_str())!=Some("Versions")||versions.parent().and_then(|p|p.file_name()).and_then(|s|s.to_str())!=Some(game_definition.id()) {return Err(err())}
 xodus::secrets::init_secrets()?;let tokens=TokenManager::with_keychain_and_memory();
 let client=reqwest::Client::builder().https_only(true).connect_timeout(std::time::Duration::from_secs(30)).read_timeout(std::time::Duration::from_secs(90)).user_agent("MinecraftMacLauncher/0.2").build()?;
 if tokens.get_user().is_err(){return Err(err())}
 progress("authorizing",0,0);
 xodus::tokens::device::ensure_device_credentials(&client,&tokens).await;
 let id=crate::package::get_content_id(&client,game_definition.store_id().into(),Some("neutral".into())).await?;
 crate::license::get_license(&client,&tokens,id.clone(),"neutral".into()).await?;
 let package=crate::package::get_packages(&client,&tokens,id.clone()).await?;
 let (version, revision)=crate::package::normalize_version(&package.version)?;
 game_definition.validate(&version)?;
 if uuid::Uuid::parse_str(&package.content_id)?!=uuid::Uuid::parse_str(&id)? {return Err(err())} // Only the tested executable/profile is accepted.
 let candidates:Vec<_>=package.package_files.iter().filter(|f|f.file_name.to_ascii_lowercase().ends_with(".msixvc")).collect();
 if candidates.len()!=1{return Err(err())}let file=candidates[0];
 if file.file_size<=0||file.file_size>32*1024*1024*1024i64{return Err(err())}
 let pin_id=format!("{}:{}",id,revision.as_deref().unwrap_or(""));
 let pins:HashMap<String,String>=serde_json::from_str(include_str!("package-pins.json"))?;
 let pinned_hash=pins.get(&pin_id);
 let expected_hash=if game_definition==crate::game::Game::Bedrock {
  let digest=pinned_hash.map(String::as_str).or_else(||(!file.file_hash.is_empty()).then_some(file.file_hash.as_str()));
  if !digest.is_some_and(strong_digest) {return Err(std::io::Error::other("BEDROCK_PACKAGE_INTEGRITY_UNVERIFIED").into())}digest
 }else{if !file.file_hash.is_empty(){Some(file.file_hash.as_str())}else{pinned_hash.map(String::as_str)}};
 // Developer-only first pin acquisition over authenticated Microsoft HTTPS. Never enabled in distributable builds.
 if expected_hash.is_none() && (!cfg!(feature="bootstrap-pins") || game_definition != crate::game::Game::Dungeons) {return Err(err())}
 let file_id=uuid::Uuid::parse_str(&file.content_id)?;
 let (device_key,license)=crate::license::get_license(&client,&tokens,file.content_id.clone(),"neutral".into()).await?;
 if license.content_keys.len()!=1{return Err(err())}
 let content_key=license.content_keys.values().next().ok_or_else(err)?.unpack(&device_key)?;
 let key=*content_key;
 let game=stage.join("Game");let components=stage.join("Components");new_dir(&game)?;new_dir(&components)?;
 let encrypted=stage.join("download.msixvc");
 progress("downloading-game",0,file.file_size as u64);
 let game_client=reqwest::Client::builder().connect_timeout(std::time::Duration::from_secs(30)).read_timeout(std::time::Duration::from_secs(90)).user_agent("MinecraftMacLauncher/0.2").build()?;
 let mut package_hash=None;
 for root in file.cdn_root_paths.iter().take(4) {
  let url=format!("{}{}",root,file.relative_url);
  match download(&game_client,&url,&encrypted,Some(file.file_size as u64),file.file_size as u64,expected_hash,"downloading-game").await {
   Ok(digest)=>{package_hash=Some(digest);break;},
   Err(_)=>{if encrypted.exists(){std::fs::remove_file(&encrypted)?;}}
  }
 }
 let package_hash=package_hash.ok_or_else(err)?;
 // The developer pin bootstrap is anchored to the untouched reference's complete header/hash-table prefix.
 if expected_hash.is_none() {
  let mut prefix_file=std::fs::File::open(&encrypted)?;let mut h=Sha256::new();let mut left=63122179usize;let mut b=[0u8;1024*1024];
  while left>0 {let n=prefix_file.read(&mut b[..left.min(1024*1024)])?;if n==0{return Err(err())}h.update(&b[..n]);left-=n;}
  if hex(h.finalize())!="5daa5f84d81bd12d295fdbe66306dd22465874dec45aa68890a180feb80c4948"{return Err(err())}
 }
 let mut source=tokio::io::BufReader::with_capacity(2*1024*1024,tokio::fs::File::open(&encrypted).await?);
 let xvd=msixvc::xvd::XvdFile::parse(&mut source).await?;
 if xvd.content_id()!=file_id{return Err(err())}
 if expected_hash.is_none() {
  let mut clock=std::time::Instant::now();
  xvd.verify_all_data(&mut source,|n,total|{if clock.elapsed().as_millis()>250{progress("verifying-package",n,total);clock=std::time::Instant::now();}}).await?;
 }
 let mut integrity=create(&stage.join("package-integrity.json"))?;
 integrity.write_all(&serde_json::to_vec(&serde_json::json!({"schema":1,"packagePin":pin_id,"packageHash":package_hash,"size":file.file_size}))?)?;integrity.sync_all()?;
 progress("decrypting",0,0);
 let mut files=HashMap::new();for (name,metadata) in xvd.parse_user_package_files(&mut source).await?{
  if name=="SegmentMetadata.bin"{files.extend(xvd.parse_segment_metadata(&mut source,&metadata).await?);}
 }
 files.extend(xvd.parse_ntfs_segment_metadata(&mut source,!files.is_empty()).await?);
 if files.is_empty()||files.len()>100000{return Err(err())}
 let mut total=0u64;let mut unique=HashSet::new();let mut paths=vec![];
 for (name,file) in files {let path=safe_relative(&name)?;if !unique.insert(path.to_string_lossy().to_lowercase()){return Err(err())}total=total.checked_add(file.length).ok_or_else(err)?;paths.push((path,file));}
 if total>64*1024*1024*1024{return Err(err())}
 let mut completed=0;let mut clock=std::time::Instant::now();
 paths.sort_by(|a,b|a.0.cmp(&b.0));
 for (path,file) in paths {
  let target=game.join(path);std::fs::create_dir_all(target.parent().ok_or_else(err)?)?;
  let mut output=tokio::io::BufWriter::with_capacity(2*1024*1024,tokio::fs::File::from_std(create(&target)?));
  xvd.extract_file(&mut source,&mut output,&file,key,|n,_|{if clock.elapsed().as_millis()>250{progress("decrypting",completed+n,total);clock=std::time::Instant::now();}}).await?;
  output.flush().await?;output.get_ref().sync_all().await?;
  if output.get_ref().metadata().await?.len()!=file.length{return Err(err())}completed+=file.length;
 }
 progress("downloading-components",0,0);
 let nuget=stage.join("gdk2604.zip");
 download(&client,NUGET_URL,&nuget,Some(141330356),200*1024*1024,Some("ba9cb693a7898d921116d0a7a9bd1c37d12c55a2c25ac88b08ae17a49e135c3e"),"downloading-components").await?;extract_nuget(&nuget,&components)?;
 let sdk=stage.join("gdk2504.zip");
 if game_definition==crate::game::Game::Dungeons {
 let sdk=stage.join("gdk2504.zip");download(&client,SDK_URL,&sdk,Some(566000959),600*1024*1024,Some(SDK_HASH),"downloading-components").await?;
 extract_sdk(&sdk,&components)?;
 let vc=components.join("VC_redist.x64.exe");
 download(&client,"https://download.visualstudio.microsoft.com/download/pr/ebdab8e5-1d7b-4d9f-a11b-cbb1720c3b12/843068991DAAA1F73AD9F6239BCE4D0F6A07A51F18C37EA2A867E9BECA71295C/VC_redist.x64.exe",&vc,Some(18731856),32*1024*1024,Some("843068991daaa1f73ad9f6239bce4d0f6a07a51f18c37ea2a867e9beca71295c"),"downloading-components").await?;
 } else {
  let curl=stage.join("curl.zip");
  download(&client,"https://curl.se/windows/dl-8.22.0_2/curl-8.22.0_2-win64-mingw.zip",&curl,None,64*1024*1024,Some("7c8c6b953b4eb2953d2bdc08cca1d5f09a964e9f86c361693559400c9a6d6db0"),"downloading-components").await?;
  let mut zip=zip::ZipArchive::new(std::fs::File::open(&curl)?)?;
  for (member,target) in [("bin/libcurl-x64.dll","XCurl.dll"),("bin/curl-ca-bundle.crt","curl-ca-bundle.crt"),("COPYING.txt","curl-COPYING.txt")] {
   let mut entry=zip.by_name(&format!("curl-8.22.0_2-win64-mingw/{member}"))?;
   if entry.size()>32*1024*1024{return Err(err())}let mut out=create(&components.join(target))?;std::io::copy(&mut entry,&mut out)?;out.sync_all()?;
  }
  std::fs::remove_file(curl)?;
 }
 let mut hashes=HashMap::new();inventory(&game,&game,&mut hashes)?;inventory(&components,stage,&mut hashes)?;
 let mut receipt=create(&stage.join("download-receipt.json"))?;receipt.write_all(&serde_json::to_vec(&serde_json::json!({"schema":1,"gameId":game_definition.id(),"storeId":game_definition.store_id(),"materialization":"persistent-ssd","version":version,"revision":revision,"packagePin":pin_id,"packageHash":package_hash,"hashes":hashes}))?)?;receipt.sync_all()?;
 // Large encrypted/SDK artifacts are strictly transaction-owned and no longer needed.
 for file in [encrypted,sdk,nuget]{if file.exists(){std::fs::remove_file(file)?;}}
 println!("{}",serde_json::json!({"schema":1,"type":"complete","version":version}));Ok(())
}
fn inventory(root:&Path,base:&Path,out:&mut HashMap<String,String>)->Result<()> {
 for entry in std::fs::read_dir(root)?{let p=entry?.path();let m=std::fs::symlink_metadata(&p)?;if m.file_type().is_symlink(){return Err(err())}if m.is_dir(){inventory(&p,base,out)?}else if m.is_file(){out.insert(p.strip_prefix(base)?.to_str().ok_or_else(err)?.replace('\\',"/"),hash(&p)?);}}
 Ok(())
}
#[cfg(test)]mod tests{use super::*;#[test]fn reject_archive_escape(){for bad in ["../secret","C:\\secret","/root","a//b","a/./b","a\\..\\b"]{assert!(safe_relative(bad).is_err())}assert_eq!(safe_relative("Dungeons\\Binaries\\WinGDK\\game.exe").unwrap(),PathBuf::from("Dungeons/Binaries/WinGDK/game.exe"));}}

/// Build-time header extraction from the same pinned official SDK. Not used by end users.
pub fn extract_headers(archive:&Path,out:&Path)->Result<()> {
 if hash(archive)?!=SDK_HASH{return Err(err())}
 std::fs::create_dir_all(out)?;
 let mut zip=zip::ZipArchive::new(std::fs::File::open(archive)?)?;
 let mut ids=HashSet::new();let mut cabs=vec![];
 for i in 0..zip.len(){let mut entry=zip.by_index(i)?;let name=entry.name().to_string();
  if name.ends_with(".msi")&&entry.size()<64*1024*1024{
   let mut bytes=vec![];entry.read_to_end(&mut bytes)?;let mut m=msi::Package::open(std::io::Cursor::new(bytes))?;
   if m.has_table("File"){for row in m.select_rows(msi::Select::table("File"))? {
    if row["FileName"].as_str().is_some_and(|s|s.split('|').next_back()==Some("XCurl.h")){if let Some(id)=row["File"].as_str(){ids.insert(id.to_string());}}
   }}
  }else if name.ends_with(".cab"){cabs.push(name);}
 }
 for name in cabs{let mut entry=zip.by_name(&name)?;if entry.size()>600*1024*1024{return Err(err())}let mut bytes=vec![];entry.read_to_end(&mut bytes)?;
  let mut cab=cab::Cabinet::new(std::io::Cursor::new(bytes))?;
  let files:Vec<_>=cab.folder_entries().flat_map(|f|f.file_entries()).filter(|f|ids.contains(f.name())&&f.uncompressed_size()<1024*1024).map(|f|f.name().to_string()).collect();
  for id in files{let path=out.join("XCurl.h");if path.exists(){continue;}let mut file=create(&path)?;std::io::copy(&mut cab.read_file(&id)?,&mut file)?;file.sync_all()?;}
 }
 if !out.join("XCurl.h").exists(){return Err(err())}Ok(())
}

fn strong_digest(value:&str)->bool {
 (value.len()==64&&value.bytes().all(|b|b.is_ascii_hexdigit())) || base64::engine::general_purpose::STANDARD.decode(value).is_ok_and(|v|v.len()==32)
}
#[cfg(test)]mod digest_tests {
 use super::*;
 #[test]fn bedrock_requires_full_sha256_anchor(){assert!(strong_digest(&"ab".repeat(32)));assert!(strong_digest(&base64::engine::general_purpose::STANDARD.encode([1u8;32])));assert!(!strong_digest(&"ab".repeat(20)));assert!(!strong_digest(""));assert!(!strong_digest("untrusted"));}
}

/// Engineering-only public trust-anchor inspection of our pinned official SDK.
/// Never writes private keys, executables or redistributable binaries.
#[cfg(feature="bootstrap-pins")]
pub fn extract_public_signing_keys(archive:&Path,out:&Path)->Result<()> {
 if hash(archive)?!=SDK_HASH { return Err(err()); }
 let mut zip=zip::ZipArchive::new(std::fs::File::open(archive)?)?;
 let mut targets=HashSet::new();let mut cabinets=vec![];
 for i in 0..zip.len() {
  let mut entry=zip.by_index(i)?;let name=entry.name().to_owned();
  if !name.contains("Installers/") {continue;}
  if name.ends_with(".msi") && entry.size()<64*1024*1024 {
   let mut bytes=vec![];entry.read_to_end(&mut bytes)?;let mut msi=msi::Package::open(std::io::Cursor::new(bytes))?;
   if !msi.has_table("File") {continue;}
   for row in msi.select_rows(msi::Select::table("File"))? {
    let name=row["FileName"].as_str().unwrap_or("").split('|').next_back().unwrap_or("").to_ascii_lowercase();
    if ["makepkg", "packageutil", "xcihash", "xvdd", "packager", "xsapi"].iter().any(|s|name.contains(s)) {
     println!("{}",serde_json::json!({"sdkTool":name}));
     if let Some(id)=row["File"].as_str(){targets.insert(id.to_owned());}
    }
   }
  } else if name.ends_with(".cab") {cabinets.push(name);}
 }
 for name in cabinets {
  let mut entry=zip.by_name(&name)?;if entry.size()>600*1024*1024{return Err(err());}
  let mut bytes=vec![];entry.read_to_end(&mut bytes)?;let mut cab=cab::Cabinet::new(std::io::Cursor::new(bytes))?;
  let names:Vec<_>=cab.folder_entries().flat_map(|f|f.file_entries()).filter(|f| targets.contains(f.name()) && f.uncompressed_size()<64*1024*1024).map(|f|f.name().to_owned()).collect();
  for name in names {
   let mut bytes=vec![];cab.read_file(&name)?.read_to_end(&mut bytes)?;
   println!("{}",serde_json::json!({"sdkToolBytes":bytes.len(),"rsaPublicMarkers":bytes.windows(4).filter(|b|*b==b"RSA1").count()}));
   for i in 0..bytes.len().saturating_sub(0x21b) {
    if &bytes[i..i+4]!=b"RSA1" {continue;}
    let blob=&bytes[i..i+0x21b];let hash=hex(Sha256::digest(blob));
    let name=match hash.as_str() {
     "618c5fb1193040af8bc1c0199b850b4b5c42e43ce388129180284e4ef0b18082"=>"GreenXvdPublicKey",
     "183f0ae05431e4ad91554e88946967c872997227dbe6c85116f5fd2fd2d1229e"=>"GreenGamesPublicKey",
     _=>continue,
    };
    let path=out.join(format!("{name}.rsa"));if !path.exists(){create(&path)?.write_all(blob)?;}
    println!("{}",serde_json::json!({"publicKey":name,"sha256":hash}));
   }
  }
 }
 Ok(())
}
