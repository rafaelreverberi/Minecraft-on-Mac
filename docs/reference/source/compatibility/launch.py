#!/usr/bin/env python3
"""Direct CrossOver launch. Discard raw stdout; retain only known DLL resolution."""
from pathlib import Path
import os, subprocess, re, sys
ROOT=Path(__file__).resolve().parent
HOME=Path.home(); BIN=HOME/'Games/MinecraftDungeons2-FullyDecrypted/Dungeons/Binaries/WinGDK'
wines=[HOME/'Applications/CrossOver.app/Contents/SharedSupport/CrossOver/CrossOver-Hosted Application/wine',Path('/Applications/CrossOver.app/Contents/SharedSupport/CrossOver/CrossOver-Hosted Application/wine')]
wine=next((p for p in wines if p.exists()),None)
if not wine: raise SystemExit('CrossOver wine executable missing')
env=os.environ.copy()
for k in ['WINE_DLL_FILE_MAP','XGR_NO_DELEGATE','RUST_LOG','XODUS_LOG']: env.pop(k,None)
env['XCURL_COMPAT_IDENTITY']='0' if '--baseline' in sys.argv else '1'
env['WINEDLLOVERRIDES']='xgameruntime=n;XCurl=n'
env['WINEDEBUG']='+loaddll' if '--loader' in sys.argv else '-all'
with (ROOT/'loader.log').open('w') as log:
 p=subprocess.Popen([str(wine),'--debugmsg',env['WINEDEBUG'],'--bottle','Dungeons2Xbox','--dll',env['WINEDLLOVERRIDES'],'--workdir',str(BIN),str(BIN/'Dungeons-WinGDK-Shipping.exe')],env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,errors='replace')
 for line in p.stdout:
  # No generic Wine output reaches disk or terminal. Only static module paths.
  if 'loaddll:' in line and 'Loaded L"' in line:
   m=re.search(r'Loaded L"([^"\r\n]+)"',line)
   if m and any(m.group(1).lower().endswith(n) for n in ['xcurl.dll','xcurl2504.dll','xgameruntime.dll','xgameruntime.gdk.dll','libhttpclient.gdk.dll']):
    text='loaded '+m.group(1);log.write(text+'\n');log.flush();print(text,flush=True)
 sys.exit(p.wait())
