#!/usr/bin/env python3
"""Build a private arm64 preview; no runtime compiler requirements or Microsoft DLLs."""
from pathlib import Path
import subprocess, shutil, hashlib, plistlib, json, os, argparse
root=Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--reuse-compatibility',action='store_true',help='Use previously built artifacts only after verifying every recorded digest.')
args=parser.parse_args()
if args.reuse_compatibility:
 resources=root/'Sources/LauncherCore/Resources'
 checks=[]
 managed=json.loads((resources/'compatibility-managed.json').read_text())
 for name in ['XCurl.dll','xgameruntime.dll']:
  checks.append((resources/'Managed'/name,managed['hashes']['Dungeons/Binaries/WinGDK/'+name]))
 checks.extend((resources/'Probes'/name,digest) for name,digest in json.loads((resources/'probe-sha256.json').read_text()).items())
 checks.extend((resources/'Bedrock'/name,digest) for name,digest in json.loads((resources/'Bedrock/provenance.json').read_text())['hashes'].items())
 for path,digest in checks:
  if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest()!=digest:
   raise SystemExit('Cannot reuse missing or changed compatibility artifact: '+str(path))
 print('Verified and retained existing compatibility artifacts without rebuilding them.')
else:
 subprocess.run(['python3',str(root/'scripts/build_bedrock.py')],check=True)
helper_build=['cargo','build','--release','--locked','--manifest-path',str(root/'NativeHelper/Cargo.toml')]
if os.environ.get('MML_BOOTSTRAP_PINS')=='1':raise SystemExit('Release packaging refuses the engineering bootstrap feature.')
subprocess.run(helper_build,check=True)
if not args.reuse_compatibility:
 subprocess.run(['python3',str(root/'scripts/build_probes.py')],check=True)
 subprocess.run(['python3',str(root/'scripts/build_compatibility.py')],check=True)
helper=root/'NativeHelper/target/release/minecraft-native-helper'
# Sign helper first because signing changes its digest.
subprocess.run(['codesign','--force','--sign','-',str(helper)],check=True)
(root/'Sources/LauncherCore/Resources/helper-sha256.txt').write_text(hashlib.sha256(helper.read_bytes()).hexdigest()+'\n')
subprocess.run(['swift','build','-c','release'],cwd=root,check=True)
bin_dir=Path(subprocess.check_output(['swift','build','-c','release','--show-bin-path'],cwd=root,text=True).strip())
app=root/'build/Minecraft on Mac.app'
if app.exists():shutil.rmtree(app)
for folder in ['MacOS','Resources','Helpers']: (app/'Contents'/folder).mkdir(parents=True,exist_ok=True)
shutil.copy2(bin_dir/'MinecraftMacLauncher',app/'Contents/MacOS/MinecraftMacLauncher')
shutil.copy2(helper,app/'Contents/Helpers/minecraft-native-helper')
restored_bedrock_bundles=0
for resource in bin_dir.glob('*.bundle'):
 shutil.copytree(resource,app/'Contents/Resources'/resource.name)
 # Xcode's Release copy phase strips Mach-O .so resources despite SwiftPM .copy.
 # Restore the independently hashed Bedrock artifact bytes before signing.
 bedrock_source=root/'Sources/LauncherCore/Resources/Bedrock'
 provenance=json.loads((bedrock_source/'provenance.json').read_text())
 for destination in (app/'Contents/Resources'/resource.name).rglob('Bedrock/provenance.json'):
  restored_bedrock_bundles+=1
  for name,digest in provenance['hashes'].items():
   source=bedrock_source/name
   if hashlib.sha256(source.read_bytes()).hexdigest()!=digest:
    raise SystemExit('Bedrock source artifact differs from provenance: '+name)
   shutil.copy2(source,destination.parent/name)
  # Re-sign the resource bundle after restoring its hashed resource bytes.
  subprocess.run(['codesign','--force','--sign','-',str(app/'Contents/Resources'/resource.name)],check=True)
if restored_bedrock_bundles!=1:raise SystemExit('Expected exactly one Bedrock runtime resource bundle.')
notices=app/'Contents/Resources/Notices';notices.mkdir()
for source,name in [('Compatibility/Bedrock/LICENSE','WineGDK-LGPL-LICENSE'),('docs/BEDROCK.md','BEDROCK.md'),('LICENSE','LICENSE'),('docs/VENDOR.md','VENDOR.md'),('docs/MANAGED-INSTALLATION.md','MANAGED-INSTALLATION.md'),('Compatibility/Runtime/LICENSE','Runtime-MIT-LICENSE'),('Compatibility/XCurl/LICENSE','XCurl-MIT-LICENSE')]:
 shutil.copy2(root/source,notices/name)
subprocess.run(['python3',str(root/'scripts/collect_notices.py'),str(notices)],check=True)
# Xcode's SwiftPM engine produces resource .bundle directories alongside executable.
info={'CFBundleIdentifier':'org.minecraftmac.launcher.preview','CFBundleName':'Minecraft on Mac','CFBundleDisplayName':'Minecraft on Mac','CFBundleExecutable':'MinecraftMacLauncher','CFBundleIconFile':'LauncherMac','CFBundlePackageType':'APPL','CFBundleShortVersionString':'0.3.0','CFBundleVersion':'4','LSMinimumSystemVersion':'14.0','NSHighResolutionCapable':True,'NSPrincipalClass':'NSApplication','LSApplicationCategoryType':'public.app-category.games'}
shutil.copy2(root/'Sources/MinecraftMacLauncher/Resources/Artwork/LauncherMac.icns',app/'Contents/Resources/LauncherMac.icns')
(app/'Contents/Info.plist').write_bytes(plistlib.dumps(info))
subprocess.run(['codesign','--force','--sign','-',str(app)],check=True)
subprocess.run(['codesign','--verify','--deep','--strict',str(app)],check=True)
# Verify delivered bytes after all Swift/Xcode copy phases and signing operations.
for manifest in app.rglob('Bedrock/provenance.json'):
 for name,digest in json.loads(manifest.read_text())['hashes'].items():
  if hashlib.sha256((manifest.parent/name).read_bytes()).hexdigest()!=digest:
   raise SystemExit('Packaged Bedrock artifact changed: '+name)
print('Packaged Bedrock provenance matches every delivered runtime artifact.')
print(app)
