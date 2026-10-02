#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
int main(void){
 HMODULE m=LoadLibraryA("XCurl.dll");if(!m){printf("PROBE load win32=%lu\n",GetLastError());return 1;}
 const char*names[]={"curl_easy_cleanup","curl_easy_duphandle","curl_easy_escape","curl_easy_getinfo","curl_easy_init","curl_easy_perform","curl_easy_reset","curl_easy_setopt","curl_easy_strerror","curl_easy_unescape","curl_escape","curl_formadd","curl_formfree","curl_formget","curl_free","curl_getdate","curl_global_cleanup","curl_global_init","curl_global_init_mem","curl_global_sslset","curl_mime_addpart","curl_mime_data","curl_mime_data_cb","curl_mime_encoder","curl_mime_filedata","curl_mime_filename","curl_mime_free","curl_mime_headers","curl_mime_init","curl_mime_name","curl_mime_subparts","curl_mime_type","curl_multi_add_handle","curl_multi_cleanup","curl_multi_info_read","curl_multi_init","curl_multi_perform","curl_multi_poll","curl_multi_remove_handle","curl_multi_setopt","curl_multi_strerror","curl_multi_wait","curl_multi_wakeup","curl_share_cleanup","curl_share_init","curl_share_setopt","curl_share_strerror","curl_slist_append","curl_slist_free_all","curl_unescape","curl_url","curl_url_cleanup","curl_url_dup","curl_url_get","curl_url_set","curl_version","curl_version_info","xcurl_global_init_mem","xcurl_global_resume","xcurl_global_set_request_limit","xcurl_global_suspend"};
 for(unsigned i=0;i<sizeof(names)/sizeof(names[0]);i++)if(!GetProcAddress(m,names[i]) || GetProcAddress(m,names[i])!=GetProcAddress(m,(const char*)(uintptr_t)(i+1))){printf("PROBE missing export index=%u\n",i);return 2;}
 void*(*init)(void)=(void*)GetProcAddress(m,"curl_easy_init");void(*cleanup)(void*)=(void*)GetProcAddress(m,"curl_easy_cleanup");
 void*easy=init();if(easy){int(*set)(void*,int,... )=(void*)GetProcAddress(m,"curl_easy_setopt");printf("PROBE before compression option\n");fflush(stdout);int r=set(easy,10102,"");printf("PROBE compression option result=%d\n",r);if(r!=0)return 4; r=set(easy,10102,"gzip, deflate"); if(r!=0)return 5; cleanup(easy);}
 printf("PROBE exports OK easyHandle=%d\n",easy!=NULL);return easy?0:3;
}
