#include "../contracts/license_guard.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>
#include <pthread.h>
static const uint8_t request[]="{\"schema_version\":1,\"license_path\":\"/nonexistent/license.lic\",\"identity_path\":\"/nonexistent/installation.json\",\"product\":\"worker-suite\",\"required_features\":[\"messaging\"]}";
static void *check(void *arg){
    (void)arg;
    struct { unsigned char a[16]; unsigned char out[16384]; unsigned char b[16]; } buffer;
    memset(&buffer,0xa5,sizeof buffer);size_t n=999;
    assert(lg_validate_v1(request,sizeof request-1,buffer.out,sizeof buffer.out,&n)==0);
    assert(n>0&&n<sizeof buffer.out);
    assert(strstr((char*)buffer.out,"\"valid\":false")!=NULL);
    for(size_t i=0;i<16;i++)assert(buffer.a[i]==0xa5&&buffer.b[i]==0xa5);
    return NULL;
}
int main(void){
    assert(lg_abi_version()==1);size_t n=999;unsigned char output[16];memset(output,0xa5,sizeof output);
    assert(lg_validate_v1(NULL,1,output,sizeof output,&n)==-1&&n==0);
    assert(lg_validate_v1(request,sizeof request-1,output,sizeof output,NULL)==-1);
    assert(lg_validate_v1(request,sizeof request-1,NULL,1,&n)==-1&&n==0);
    assert(lg_validate_v1(request,sizeof request-1,output,sizeof output,&n)==-2&&n>16);
    for(size_t i=0;i<16;i++)assert(output[i]==0xa5);
    assert(lg_validate_v1(request,sizeof request-1,NULL,0,&n)==-2&&n>16);
    assert(lg_validate_v1((const uint8_t*)"{}",2,output,sizeof output,&n)==-1&&n==0);
    pthread_t threads[8];for(int i=0;i<8;i++)assert(pthread_create(&threads[i],NULL,check,NULL)==0);
    for(int i=0;i<8;i++)assert(pthread_join(threads[i],NULL)==0);
    puts("PASS: ABI version, pointer/length checks, zero return denial, sizing, canaries, concurrency");
    return 0;
}
