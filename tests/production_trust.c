#include "../contracts/license_guard.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>
int main(void){
    const char *request="{\"schema_version\":1,\"license_path\":\"/trusted/valid.lic\",\"identity_path\":\"/trusted/installation.json\",\"product\":\"worker-suite\",\"required_features\":[\"messaging\"]}";
    unsigned char output[16384];size_t written=0;
    assert(lg_validate_v1((const uint8_t*)request,strlen(request),output,sizeof output-1,&written)==0);
    assert(written<sizeof output);output[written]=0;
    assert(strstr((char*)output,"\"valid\":false"));
    assert(strstr((char*)output,"\"code\":\"UNKNOWN_KEY\""));
    puts("PASS: production native artifact rejects fixture issuer; development environment overrides ignored");
    return 0;
}
