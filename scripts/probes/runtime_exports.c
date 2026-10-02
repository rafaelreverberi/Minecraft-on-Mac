/* Load-only probe: never invokes account APIs or prints account data. */
#include <windows.h>
int main(void) {
    HMODULE m=LoadLibraryA("xgameruntime.dll"); if(!m)return 1;
    const char *names[]={"DllCanUnloadNow","InitializeApiImpl","InitializeApiImplEx","InitializeApiImplEx2","QueryApiImpl","UninitializeApiImpl","XErrorReport"};
    for(unsigned i=0;i<sizeof(names)/sizeof(names[0]);i++)if(!GetProcAddress(m,names[i]))return 2;
    return 0;
}
