/* SPDX-License-Identifier: AGPL-3.0-only */
#include "nextime.h"
#include <stdio.h>
#include <string.h>

#define CHECK(call) do { int32_t status=(call); if(status!=NEXTIME_OK) { fprintf(stderr,"status=%d at line %d\n",status,__LINE__); return 1; } } while(0)
int main(void) {
    uint64_t handle=0;
    uint32_t flags=0, count=0, selected=0;
    uint8_t buffer[512];
    size_t needed=0;
    if(nextime_abi_version()!=1) return 1;
    CHECK(nextime_session_create(NULL,0,8,&handle));
    const char *input="hashi";
    for(size_t i=0;i<strlen(input);++i) {
        CHECK(nextime_session_event(handle,NEXTIME_TEXT,(const uint8_t*)input+i,1,&flags));
        if(flags!=NEXTIME_HANDLED) return 1;
    }
    CHECK(nextime_session_event(handle,NEXTIME_CONVERT,NULL,0,&flags));
    CHECK(nextime_session_candidates(handle,&count,&selected));
    if(count!=4 || selected!=0) return 1;
    for(uint32_t i=0;i<count;++i) {
        CHECK(nextime_session_text(handle,NEXTIME_CANDIDATE,i,buffer,sizeof(buffer),&needed));
        printf("candidate %u: %s\n",i+1,(char*)buffer);
    }
    CHECK(nextime_session_event(handle,NEXTIME_NEXT,NULL,0,&flags));
    CHECK(nextime_session_event(handle,NEXTIME_ENTER,NULL,0,&flags));
    if(flags!=(NEXTIME_HANDLED|NEXTIME_HAS_COMMIT)) return 1;
    CHECK(nextime_session_text(handle,NEXTIME_COMMITTED,0,buffer,sizeof(buffer),&needed));
    /* U+7BB8 (箸), checked without depending on the C compiler's source encoding. */
    if(strcmp((char*)buffer,"\xe7\xae\xb8")!=0) return 1;
    printf("commit: %s\n",(char*)buffer);
    CHECK(nextime_session_destroy(handle));
    if(nextime_session_destroy(handle)!=NEXTIME_INVALID_HANDLE) return 1;
    return 0;
}
