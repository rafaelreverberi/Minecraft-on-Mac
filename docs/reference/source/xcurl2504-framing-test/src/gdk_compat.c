/* Minimal GDK XCurl compatibility: negotiate uncompressed HTTP responses
 * only when the local WinHTTP lacks response-decompression support.
 * All 60 other exports forward directly to the genuine Microsoft DLL. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <winhttp.h>
#include <stdarg.h>
#include <stdint.h>
#include <wchar.h>
#define CURL_STATICLIB
#include "XCurl.h"
static HMODULE self,backend;
static INIT_ONCE once=INIT_ONCE_STATIC_INIT;
static int no_decompression;
static BOOL CALLBACK initialize(PINIT_ONCE once_arg,PVOID parameter,PVOID *context){
 wchar_t path[32768];DWORD n=GetModuleFileNameW(self,path,32768);
 if(!n||n>=32768)return FALSE;
 wchar_t *slash=wcsrchr(path,L'\\');if(!slash)return FALSE;
 wcscpy(slash+1,L"XCurl2504.dll");backend=LoadLibraryW(path);if(!backend)return FALSE;
 wchar_t flag[4];int disabled=GetEnvironmentVariableW(L"XCURL_COMPAT_IDENTITY",flag,4)==1&&flag[0]==L'0';
 if(!disabled){
  /* Capability check only: no request is sent and no DNS lookup occurs. */
  HINTERNET session=WinHttpOpen(L"XCurl compatibility capability check",WINHTTP_ACCESS_TYPE_NO_PROXY,NULL,NULL,0);
  HINTERNET conn=session?WinHttpConnect(session,L"localhost",80,0):NULL;
  HINTERNET request=conn?WinHttpOpenRequest(conn,L"GET",L"/",NULL,NULL,NULL,0):NULL;
  if(request){DWORD mask=WINHTTP_DECOMPRESSION_FLAG_ALL;SetLastError(0);
   BOOL supported=WinHttpSetOption(request,WINHTTP_OPTION_DECOMPRESSION,&mask,sizeof(mask));
   no_decompression=!supported&&GetLastError()==ERROR_WINHTTP_INVALID_OPTION;
   WinHttpCloseHandle(request);
  }
  if(conn)WinHttpCloseHandle(conn);if(session)WinHttpCloseHandle(session);
 }
 return TRUE;
}
CURLcode curl_easy_setopt(CURL *easy,CURLoption option,...){
 if(!InitOnceExecuteOnce(&once,initialize,NULL,NULL))return CURLE_FAILED_INIT;
 CURLcode (*set)(CURL*,CURLoption,... )=(void*)GetProcAddress(backend,"curl_easy_setopt");
 if(!set)return CURLE_FAILED_INIT;
 va_list args;va_start(args,option);CURLcode result;
 if(option<10000){long value=va_arg(args,long);result=set(easy,option,value);}
 else if(option>=30000&&option<40000){curl_off_t value=va_arg(args,curl_off_t);result=set(easy,option,value);}
 else {void *value=va_arg(args,void*);
  /* GDK 2504 is not standard libcurl here: its empty string clears the
   * decompression mask, whereas NULL falls through to strstr(NULL,...).
   * No caller header list, request bytes, callback, or TLS option is edited. */
  if(no_decompression&&option==CURLOPT_ACCEPT_ENCODING&&value)value=(void*)"";
  result=set(easy,option,value);
 }
 va_end(args);return result;
}
BOOL WINAPI DllMain(HINSTANCE instance,DWORD reason,LPVOID reserved){
 if(reason==DLL_PROCESS_ATTACH){self=instance;DisableThreadLibraryCalls(instance);}return TRUE;
}
