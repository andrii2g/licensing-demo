#include "../contracts/license_guard.h"
#include <string.h>
uint32_t lg_abi_version(void) {
#ifdef BAD_ABI
    return 999;
#else
    return 1;
#endif
}
int32_t lg_validate_v1(const uint8_t *request,size_t request_len,uint8_t *output,size_t capacity,size_t *length) {
    (void)request;(void)request_len;
    if (!length) return -1;
    *length=1;
    if (!output || capacity<1) return -2;
    output[0]='{';
    return 0;
}
