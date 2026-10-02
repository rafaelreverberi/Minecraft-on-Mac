#!/usr/bin/env python3
"""Reversible installation and sanitized state report for this GDK game only."""
from pathlib import Path
import subprocess,os,signal,shutil,json,hashlib,time,sys
ROOT=Path(__file__).resolve().parent;WORK=ROOT.parent;HOME=Path.home()
GAME=HOME/'Games/MinecraftDungeons2-FullyDecrypted';BIN=GAME/'Dungeons/Binaries/WinGDK'
BOTTLE=HOME/'Library/Application Support/CrossOver/Bottles/Dungeons2Xbox'
BACKUP=Path((WORK/'compat-backups/latest.txt').read_text().strip())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def game_pids():
 out=[]
 for row in subprocess.check_output(['ps','-axo','pid,args'],text=True).splitlines():
  fields=row.strip().split(None,1)
  if len(fields)!=2 or not fields[0].isdigit():continue
  if fields[1].startswith('Y:\\Games\\MinecraftDungeons2-FullyDecrypted\\Dungeons\\Binaries\\WinGDK\\Dungeons-WinGDK-Shipping.exe'):out.append(int(fields[0]))
 return out
def stop():
 for pid in game_pids():os.kill(pid,signal.SIGTERM)
 deadline=time.monotonic()+10
 while game_pids() and time.monotonic()<deadline:time.sleep(.2)
 if game_pids():raise SystemExit('Game still running; quit it before changing DLLs. No forced kill performed.')
command=sys.argv[1] if len(sys.argv)>1 else 'report'
if command=='stop':stop();print('Test game stopped.')
elif command in ['install-minimal','install-diagnostic']:
 if game_pids():raise SystemExit('Quit the game or run manage.py stop before installing.')
 out=WORK/'xcurl2504-framing-test/build'/('minimal' if command=='install-minimal' else '')
 # Keep the successful diagnostic build available without overwriting original backup.
 if (BIN/'XCurl.dll').exists():shutil.copy2(BIN/'XCurl.dll',ROOT/'previous-test-XCurl.dll')
 for name in ['XCurl.dll','XCurl2504.dll']:shutil.copy2(out/name,BIN/name)
 r=WORK/'xgameruntime-rs-pr12/target/x86_64-pc-windows-gnu/release/xgameruntime.dll'
 for d in [BIN,GAME,BOTTLE/'drive_c/windows/system32']:shutil.copy2(r,d/'xgameruntime.dll')
 record={str(p):sha(p) for p in [BIN/'XCurl.dll',BIN/'XCurl2504.dll',BIN/'xgameruntime.dll',GAME/'xgameruntime.dll',BOTTLE/'drive_c/windows/system32/xgameruntime.dll']}
 (ROOT/'installed.json').write_text(json.dumps(record,indent=2));print('Installed',command.removeprefix('install-'),'build. Original backup:',BACKUP)
elif command=='restore':
 if game_pids():raise SystemExit('Quit the game before restoring.')
 manifest=json.loads((BACKUP/'manifest.json').read_text());installed=json.loads((ROOT/'installed.json').read_text()) if (ROOT/'installed.json').exists() else {}
 for entry in manifest:
  p=Path(entry['path'])
  if str(p) in installed:
   if p.exists() and sha(p)!=installed[str(p)]:raise SystemExit('A managed DLL changed since installation; refusing to overwrite it.')
   shutil.copy2(entry['backup'],p)
 sidecar=BIN/'XCurl2504.dll'
 # Leave the inert backend sidecar in place: its prior existence was not recorded.
 # Restored genuine XCurl does not reference it.
 print('Restored original XCurl and runtime DLLs. Save snapshots remain in the backup; saves were not rolled back.')
elif command=='report':
 for name in ['XCurl.dll','XCurl2504.dll','libHttpClient.GDK.dll','xgameruntime.dll','xgameruntime.gdk.dll']:
  p=BIN/name;print(name,sha(p) if p.exists() else 'missing')
 print('Game running:',bool(game_pids()))
else:raise SystemExit('Usage: manage.py report|stop|install-minimal|install-diagnostic|restore')
