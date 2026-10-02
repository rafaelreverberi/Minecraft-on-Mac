#!/usr/bin/env python3
"""Release-time compilation only. Builds copied fixtures; reference stays untouched."""
from pathlib import Path
import subprocess, shutil, hashlib, json
root=Path(__file__).resolve().parents[1]
work=root/'build/probe-source'
work.mkdir(parents=True,exist_ok=True)
for relative in ['compatibility/test_token_abi.py','xgameruntime-rs-pr12/src/user.rs']:
    source=root/'docs/reference/source'/relative
    target=work/relative;target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(source,target)
subprocess.run(['python3',str(work/'compatibility/test_token_abi.py')],check=True)
out=root/'Sources/LauncherCore/Resources/Probes';out.mkdir(parents=True,exist_ok=True)
shutil.copy2(work/'compatibility/token-abi-tests.exe',out/'token-abi.exe')
for name in ['xcurl_exports','runtime_exports']:
    subprocess.run(['x86_64-w64-mingw32-gcc','-O2',str(root/'scripts/probes'/f'{name}.c'),'-o',str(out/f'{name}.exe')],check=True)
digests={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in out.glob('*.exe')}
(root/'Sources/LauncherCore/Resources/probe-sha256.json').write_text(json.dumps(digests,indent=2)+'\n')
print('Built synthetic ABI, 61 XCurl exports/decompression and load-only runtime probes.')
