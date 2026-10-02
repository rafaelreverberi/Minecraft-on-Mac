#!/usr/bin/env python3
"""Build a private arm64 preview; no runtime compiler requirements or Microsoft DLLs."""
from pathlib import Path
import subprocess, shutil, hashlib, plistlib, json
root=Path(__file__).resolve().parents[1]
subprocess.run(['cargo','build','--release','--locked','--manifest-path',str(root/'NativeHelper/Cargo.toml')],check=True)
subprocess.run(['python3',str(root/'scripts/build_probes.py')],check=True)
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
# Xcode's SwiftPM engine produces resource .bundle directories alongside executable.
info={'CFBundleIdentifier':'org.minecraftmac.launcher.preview','CFBundleName':'Minecraft on Mac','CFBundleDisplayName':'Minecraft on Mac','CFBundleExecutable':'MinecraftMacLauncher','CFBundlePackageType':'APPL','CFBundleShortVersionString':'0.1.0','CFBundleVersion':'1','LSMinimumSystemVersion':'14.0','NSHighResolutionCapable':True,'NSPrincipalClass':'NSApplication','LSApplicationCategoryType':'public.app-category.games'}
(app/'Contents/Info.plist').write_bytes(plistlib.dumps(info))
subprocess.run(['codesign','--force','--sign','-',str(app)],check=True)
subprocess.run(['codesign','--verify','--deep','--strict',str(app)],check=True)
print(app)
