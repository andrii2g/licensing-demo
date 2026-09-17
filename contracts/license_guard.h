#ifndef LICENSE_GUARD_H
#define LICENSE_GUARD_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* v1: return 0 means complete JSON response, NOT a valid license. */
uint32_t lg_abi_version(void);
int32_t lg_validate_v1(const uint8_t *request, size_t request_len,
                       uint8_t *output, size_t output_capacity,
                       size_t *output_len);
/* -1 invalid request/arguments; -2 buffer too small; -3 internal error.
 * Caller owns buffers; no trailing NUL is written. See docs/07-native-abi.md. */
#ifdef __cplusplus
}
#endif
#endif
