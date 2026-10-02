#!/usr/bin/env python3
"""Developer build only. The resulting app contains binaries, never SDK/game files."""
from pathlib import Path
import subprocess,shutil,os,hashlib,json
root=Path(__file__).resolve().parents[1]
sdk=Path(os.environ.get('MML_XCURL_INCLUDE',str(root.parent/'GDK2504-Update6/runtime/Program Files/Microsoft GDK/250406/GRDK/ExtensionLibraries/Xbox.XCurl.API/Include')))
if not (sdk/'XCurl.h').is_file():raise SystemExit('Set MML_XCURL_INCLUDE to official GDK 2504.6 XCurl headers on the build host.')
out=root/'Sources/LauncherCore/Resources/Managed';out.mkdir(parents=True,exist_ok=True)
env=os.environ.copy();env['RUSTC']=subprocess.check_output(['rustup','which','rustc'],text=True).strip();env['CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER']='x86_64-w64-mingw32-gcc'
subprocess.run(['rustup','run','stable','cargo','build','--release','--locked','--features','xuser','--target','x86_64-pc-windows-gnu','--manifest-path',str(root/'Compatibility/Runtime/Cargo.toml')],env=env,check=True)
subprocess.run(['x86_64-w64-mingw32-gcc','-shared','-O2','-Wall','-Wextra','-Wno-unused-parameter','-Wno-unknown-pragmas','-I'+str(sdk),str(root/'Compatibility/XCurl/gdk_compat.c'),str(root/'Compatibility/XCurl/curl.def'),'-o',str(out/'XCurl.dll'),'-lwinhttp','-static-libgcc'],check=True)
shutil.copy2(root/'Compatibility/Runtime/target/x86_64-pc-windows-gnu/release/xgameruntime.dll',out/'xgameruntime.dll')
profile=json.loads((root/'Sources/LauncherCore/Resources/compatibility.json').read_text());profile['id']='dungeons2-managed-1.1.1-r3';profile['revision']=3
for name in ['XCurl.dll','xgameruntime.dll']:
 profile['hashes']['Dungeons/Binaries/WinGDK/'+name]=hashlib.sha256((out/name).read_bytes()).hexdigest()
(root/'Sources/LauncherCore/Resources/compatibility-managed.json').write_text(json.dumps(profile,indent=2)+'\n')
