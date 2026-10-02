#!/usr/bin/env python3
"""Build only the GDK metadata proxy with MinGW; verify exact backend ABI."""
import os, subprocess, re, shutil, sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1];WORK=ROOT.parent
sdk=WORK/'GDK2504-Update6/runtime/Program Files/Microsoft GDK/250406/GRDK/ExtensionLibraries/Xbox.XCurl.API'
backend=Path(os.environ.get('XCURL_BACKEND',str(sdk/'Redist/x64/XCurl.dll')))
cc=shutil.which('x86_64-w64-mingw32-gcc');objdump=shutil.which('x86_64-w64-mingw32-objdump')
if not cc or not objdump:raise SystemExit('Install Homebrew mingw-w64 first')
def exports(path):
 text=subprocess.check_output([objdump,'-p',str(path)],text=True).split('[Ordinal/Name Pointer] Table')[1].split('The Function Table')[0]
 return [(int(o),n) for o,n in re.findall(r'\+base\[\s*(\d+)\]\s+[0-9a-f]+\s+(\w+)',text)]
minimal='--minimal' in sys.argv
intercept={'curl_easy_init','curl_easy_setopt','curl_easy_perform','curl_multi_add_handle','curl_multi_info_read','curl_easy_cleanup','curl_easy_reset','curl_easy_duphandle','curl_global_init','curl_global_init_mem','xcurl_global_init_mem'}
if minimal:intercept={'curl_easy_setopt'}
ex=exports(backend)
lines=['LIBRARY XCurl.dll','EXPORTS']+[f'{name}'+('' if name in intercept else f'=XCurl2504.{name}')+f' @{ordinal}' for ordinal,name in ex]
out=ROOT/'build'/('minimal' if minimal else '');out.mkdir(parents=True,exist_ok=True)
definition=out/'curl.def' if minimal else ROOT/'src/curl.def'
definition.write_text('\n'.join(lines)+'\n')
subprocess.run([cc,'-shared','-O2','-Wall','-Wextra','-Wno-unused-parameter','-Wno-unknown-pragmas','-I'+str(sdk/'Include'),str(ROOT/'src'/('gdk_compat.c' if minimal else 'gdk_probe.c')),str(definition),'-o',str(out/'XCurl.dll'),'-lwinhttp','-static-libgcc'],check=True)
assert exports(out/'XCurl.dll')==ex,'Proxy export name/ordinal mismatch'
shutil.copy2(backend,out/'XCurl2504.dll')
print(f'Built GDK {"minimal compatibility" if minimal else "metadata-only"} proxy; verified {len(ex)} names and ordinals. No auth/TLS/framing changes.')
