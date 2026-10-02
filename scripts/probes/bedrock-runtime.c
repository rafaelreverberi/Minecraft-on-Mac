/* Launcher-owned synthetic probe; no game, network, account or save operations. */
#define COBJMACROS
#include <windows.h>
#include <roapi.h>
#include <winstring.h>
#include <unknwn.h>
static const GUID factory_iid={0x00000035,0,0,{0xc0,0,0,0,0,0,0,0x46}};
int main(void) {
    const wchar_t *classes[]={L"Windows.Foundation.Metadata.ApiInformation",L"Windows.Data.Json.JsonObject",L"Windows.Data.Json.JsonArray",L"Windows.Data.Json.JsonValue",L"Windows.Storage.StorageFolder",L"Windows.Storage.ApplicationData",L"Windows.UI.Text.Core.CoreTextServicesManager"};
    HRESULT hr=RoInitialize(RO_INIT_MULTITHREADED);
    if(FAILED(hr))return 1;
    for(unsigned i=0;i<sizeof(classes)/sizeof(classes[0]);i++) {
        HSTRING text=NULL; IUnknown *factory=NULL;
        if(FAILED(WindowsCreateString(classes[i],(UINT32)wcslen(classes[i]),&text)))return 2;
        hr=RoGetActivationFactory(text,&factory_iid,(void**)&factory);
        WindowsDeleteString(text);
        if(FAILED(hr)||!factory)return 10+i;
        IUnknown_Release(factory);
    }
    HMODULE gameinput=LoadLibraryW(L"gameinput.dll");
    if(!gameinput||!GetProcAddress(gameinput,"GameInputCreate"))return 30;
    /* Check GDK export/PE linkage without initializing account networking. */
    HMODULE gdk=LoadLibraryExW(L"xgameruntime.dll",NULL,DONT_RESOLVE_DLL_REFERENCES);
    if(!gdk||!GetProcAddress(gdk,"InitializeApiImpl"))return 31;
    RoUninitialize();return 0;
}
