/* Regression probe for Wine-reset WinRT registrations. No account/game/save access. */
#define COBJMACROS
#include <windows.h>
#include <stdio.h>
#include <roapi.h>
#include <winstring.h>
#include <windows.foundation.metadata.h>
static const GUID api_iid={0x997439fe,0xf681,0x4a11,{0xb4,0x16,0xc1,0x3a,0x47,0xe8,0xba,0x36}};
int main(void) {
    HSTRING name=NULL,type=NULL,method=NULL;
    __x_ABI_CWindows_CFoundation_CMetadata_CIApiInformationStatics *api=NULL;
    boolean present=TRUE;
    if(FAILED(RoInitialize(RO_INIT_MULTITHREADED)))return 1;
    const WCHAR *class_name=L"Windows.Foundation.Metadata.ApiInformation";
    WindowsCreateString(class_name,(UINT32)wcslen(class_name),&name);
    HRESULT hr=RoGetActivationFactory(name,&api_iid,(void**)&api);
    WindowsDeleteString(name);
    if(FAILED(hr)||!api)return 2;
    const WCHAR *type_name=L"Windows.ApplicationModel.DataTransfer.DataTransferManager";
    WindowsCreateString(type_name,(UINT32)wcslen(type_name),&type);
    WindowsCreateString(L"IsSupported",11,&method);
    hr=api->lpVtbl->IsMethodPresent(api,type,method,&present);
    WindowsDeleteString(type);WindowsDeleteString(method);api->lpVtbl->Release(api);
    RoUninitialize();
    /* The owned WineGDK runtime reports unsupported optional sharing without throwing. */
    printf("API result %08lx present %u\n",(unsigned long)hr,(unsigned)present);
    return hr==S_OK&&!present ? 0 : 3;
}
