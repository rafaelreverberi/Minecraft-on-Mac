#!/usr/bin/env python3
"""Build a private arm64 preview; no runtime compiler requirements or Microsoft DLLs."""
from pathlib import Path
import subprocess, shutil, hashlib, plistlib, json, os
root=Path(__file__).resolve().parents[1]
subprocess.run(['python3',str(root/'scripts/build_bedrock.py')],check=True)
helper_build=['cargo','build','--release','--locked','--manifest-path',str(root/'NativeHelper/Cargo.toml')]
if os.environ.get('MML_BOOTSTRAP_PINS')=='1':raise SystemExit('Release packaging refuses the engineering bootstrap feature.')
subprocess.run(helper_build,check=True)
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
for resource in bin_dir.glob('*.bundle'):
 shutil.copytree(resource,app/'Contents/Resources'/resource.name)
notices=app/'Contents/Resources/Notices';notices.mkdir()
for source,name in [('Compatibility/Bedrock/LICENSE','WineGDK-LGPL-LICENSE'),('docs/BEDROCK.md','BEDROCK.md'),('LICENSE','LICENSE'),('docs/VENDOR.md','VENDOR.md'),('docs/MANAGED-INSTALLATION.md','MANAGED-INSTALLATION.md'),('Compatibility/Runtime/LICENSE','Runtime-MIT-LICENSE'),('Compatibility/XCurl/LICENSE','XCurl-MIT-LICENSE')]:
 shutil.copy2(root/source,notices/name)
subprocess.run(['python3',str(root/'scripts/collect_notices.py'),str(notices)],check=True)
# Xcode's SwiftPM engine produces resource .bundle directories alongside executable.
info={'CFBundleIdentifier':'org.minecraftmac.launcher.preview','CFBundleName':'Minecraft on Mac','CFBundleDisplayName':'Minecraft on Mac','CFBundleExecutable':'MinecraftMacLauncher','CFBundlePackageType':'APPL','CFBundleShortVersionString':'0.2.1','CFBundleVersion':'3','LSMinimumSystemVersion':'14.0','NSHighResolutionCapable':True,'NSPrincipalClass':'NSApplication','LSApplicationCategoryType':'public.app-category.games'}
(app/'Contents/Info.plist').write_bytes(plistlib.dumps(info))
subprocess.run(['codesign','--force','--sign','-',str(app)],check=True)
subprocess.run(['codesign','--verify','--deep','--strict',str(app)],check=True)
print(app)
