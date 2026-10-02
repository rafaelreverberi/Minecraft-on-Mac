#!/usr/bin/env python3
"""Build Bedrock compatibility only, from pinned source, without accessing games or accounts."""
from pathlib import Path
import subprocess, os, hashlib, json, shutil, urllib.request, tarfile
ROOT=Path(__file__).resolve().parents[1]
REV='b5d23b074cfd5e28e79acceaaefaf41a26ce6272'
DLLS=('wintypes','twinapi.appcore','windows.web','windows.storage','gameinput','windows.ui.core.textinput','windows.storage.applicationdata','windows.applicationmodel','xgameruntime')
def run(args,**kw): subprocess.run(list(map(str,args)),check=True,**kw)
def main():
    work=ROOT/'build/bedrock';work.mkdir(parents=True,exist_ok=True)
    archive=work/'winegdk-source.tar.gz'
    if not archive.exists(): urllib.request.urlretrieve(f'https://codeload.github.com/Weather-OS/WineGDK/tar.gz/{REV}',archive)
    if hashlib.sha256(archive.read_bytes()).hexdigest() != 'a99999b2ef792e4c579ac96dcf9cfb4631eb8993db701e94f4bab35f868e5373': raise SystemExit('WineGDK source archive integrity failed')
    source=work/('WineGDK-'+REV)
    if not source.exists():
        with tarfile.open(archive) as tar: tar.extractall(work,filter='data')
    patches=['macos-compatibility.patch','private-account-socket.patch']
    for name in patches:
        patch=ROOT/'Compatibility/Bedrock/patches'/name
        if subprocess.run(['git','apply','--reverse','--check',str(patch)],cwd=source,capture_output=True).returncode:
            run(['git','apply','--check',patch],cwd=source);run(['git','apply',patch],cwd=source)
    mingw=Path(os.environ.get('MML_BEDROCK_MINGW',str(Path.home()/'Downloads/llvm-mingw-20260922-ucrt-macos-universal/bin')))
    if '20260922' not in str(mingw) or not (mingw/'x86_64-w64-mingw32-gcc').is_file(): raise SystemExit('Set MML_BEDROCK_MINGW to the pinned llvm-mingw 20260922 bin directory (build host only).')
    env=dict(os.environ);env['PATH']=str(mingw)+':/opt/homebrew/opt/bison/bin:'+env.get('PATH','')
    run(['autoconf'],cwd=source,env=env)
    build=work/'winegdk';build.mkdir(exist_ok=True)
    if not (build/'Makefile').exists(): run(['arch','-x86_64','/bin/sh',source/'configure','--enable-win64','--disable-tests','--without-x','--without-freetype','--without-gnutls','CC=/usr/bin/clang -arch x86_64','CXX=/usr/bin/clang++ -arch x86_64'],cwd=build,env=env)
    run(['make','-j4',*[f'dlls/{n}/x86_64-windows/{n}.dll' for n in DLLS],'dlls/xgameruntime/xgameruntime.so'],cwd=build,env=env)
    out=ROOT/'Sources/LauncherCore/Resources/Bedrock';out.mkdir(exist_ok=True)
    hashes={}
    for name in DLLS:
        data=(build/f'dlls/{name}/x86_64-windows/{name}.dll').read_bytes()
        if name!='xgameruntime':
            if data[64:80]!=b'Wine builtin DLL': raise SystemExit('Unexpected PE marker')
            data=data[:64]+b'Local native DLL'+data[80:]
        (out/(name+'.dll')).write_bytes(data);hashes[name+'.dll']=hashlib.sha256(data).hexdigest()
    shutil.copy2(build/'dlls/xgameruntime/xgameruntime.so',out/'xgameruntime.so')
    shutil.copy2(ROOT/'Compatibility/Bedrock/classes.json',out/'classes.json')
    for name in ['xgameruntime.so','classes.json']:hashes[name]=hashlib.sha256((out/name).read_bytes()).hexdigest()
    run([mingw/'x86_64-w64-mingw32-gcc','-O2',ROOT/'scripts/probes/bedrock-runtime.c','-o',out/'bedrock-probe.exe','-lruntimeobject','-lole32'],env=env)
    hashes['bedrock-probe.exe']=hashlib.sha256((out/'bedrock-probe.exe').read_bytes()).hexdigest()
    profile={'id':'bedrock-managed-1.26.5203-r1','gameId':'bedrock','version':'1.26.5203.0','revision':1,'hashes':{n:h for n,h in hashes.items() if n.endswith('.dll')}}
    (ROOT/'Sources/LauncherCore/Resources/compatibility-bedrock.json').write_text(json.dumps(profile,indent=2)+'\n')
    manifest={'schema':1,'upstream':'https://github.com/Weather-OS/WineGDK','revision':REV,'sourceArchiveSHA256':hashlib.sha256(archive.read_bytes()).hexdigest(),'toolchain':'llvm-mingw 20260922','license':'LGPL-2.1-or-later','patches':{n:hashlib.sha256((ROOT/'Compatibility/Bedrock/patches'/n).read_bytes()).hexdigest() for n in patches},'hashes':hashes,'allocatorPatch':False}
    (out/'provenance.json').write_text(json.dumps(manifest,indent=2)+'\n')
if __name__=='__main__': main()
