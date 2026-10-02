#!/usr/bin/env python3
"""Run synthetic Windows runtime tests in a new disposable owned CrossOver bottle.
No game, account service, token store or existing environment is used.
"""
from pathlib import Path
import os, subprocess, uuid, json, shutil, sys
ROOT=Path(__file__).resolve().parents[1]
home=Path.home()
apps=[home/'Applications/CrossOver.app',Path('/Applications/CrossOver.app')]
app=next((p for p in apps if (p/'Contents/SharedSupport/CrossOver/bin/wine').is_file()),None)
if app is None:raise SystemExit('CrossOver is required for Windows runtime tests')
base=app/'Contents/SharedSupport/CrossOver'
game='bedrock' if '--bedrock' in sys.argv else 'dungeons2'
name='MinecraftMac-'+game+'-'+str(uuid.uuid4()).upper()
bottle=home/'Library/Application Support/CrossOver/Bottles'/name
if bottle.exists():raise SystemExit('Test environment unexpectedly exists; untouched')
env={k:v for k,v in os.environ.items() if k in ['HOME','USER','LOGNAME','TMPDIR','LANG','PATH']}
env['WINEDEBUG']='-all'
log=ROOT/('build/'+game+'-crossover.log')
with log.open('wb') as output:
    def run(args,timeout=180):subprocess.run(list(map(str,args)),env=env,stdout=output,stderr=output,check=True,timeout=timeout)
    run([base/'bin/cxbottle','--bottle',name,'--create','--template','win11_64','--description','Minecraft on Mac synthetic runtime tests only'])
    receipt={'schema':'2','gameId':game,'owner':name.removeprefix('MinecraftMac-'+game+'-')}
    marker=bottle/'minecraftmac-owner.json';marker.write_text(json.dumps(receipt))
    try:
        if game=='bedrock':
            runtime=ROOT/'build/bedrock-probe-runtime'
            (runtime/'x86_64-windows').mkdir(parents=True,exist_ok=True)
            (runtime/'x86_64-unix').mkdir(exist_ok=True)
            assets=ROOT/'Sources/LauncherCore/Resources/Bedrock'
            for dll in assets.glob('*.dll'):shutil.copy2(dll,runtime/'x86_64-windows'/dll.name)
            shutil.copy2(assets/'xgameruntime.so',runtime/'x86_64-unix/xgameruntime.so')
            config=bottle/'cxbottle.conf'
            search=':'.join(map(str,[runtime,base/'lib/wine/x86_64-windows',base/'lib/wine/i386-windows',base/'lib/wine']))
            config.write_text(config.read_text()+'\n[Wine]\n"DllPath" = "'+search+'"\n[EnvironmentVariables]\n"CX_GRAPHICS_BACKEND" = "d3dmetal"\n')
            registry='Windows Registry Editor Version 5.00\n\n'
            for classname,dll in json.loads((assets/'classes.json').read_text()).items():
                path='Z:'+str(runtime/'x86_64-windows'/dll).replace('/','\\')
                registry+='[HKEY_LOCAL_MACHINE\\Software\\Microsoft\\WindowsRuntime\\ActivatableClassId\\'+classname+']\n"DllPath"="'+path.replace('\\','\\\\')+'"\n\n'
            reg=runtime/'components.reg';reg.write_text(registry,encoding='utf-16')
            run([base/'bin/wine','--bottle',name,'--debugmsg','-all','regedit','/S',reg])
            overrides=','.join(dll.stem for dll in assets.glob('*.dll'))+'=n,b'
            run([base/'bin/wine','--bottle',name,'--debugmsg','-all','--dll',overrides,'--workdir',runtime/'x86_64-windows',assets/'bedrock-probe.exe'])
        else:
            exe=max((ROOT/'Compatibility/Runtime/target/x86_64-pc-windows-gnu/debug/deps').glob('xgameruntime-*.exe'),key=lambda p:p.stat().st_mtime)
            run([base/'bin/wine','--bottle',name,'--debugmsg','-all',exe,'--test-threads=1'],timeout=240)
    finally:
        if json.loads(marker.read_text())!=receipt:raise RuntimeError('Environment ownership changed; preserved')
        subprocess.run([base/'bin/wineserver','-k'],env=dict(env,WINEPREFIX=str(bottle)),stdout=output,stderr=output,check=True,timeout=30)
        subprocess.run([base/'bin/wineserver','-w'],env=dict(env,WINEPREFIX=str(bottle)),stdout=output,stderr=output,check=True,timeout=30)
        shutil.rmtree(bottle)
print('Synthetic runtime tests completed; disposable environment removed. Log:',log)
