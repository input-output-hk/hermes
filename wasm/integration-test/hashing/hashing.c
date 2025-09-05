#include "bindings_src/hermes.h"
#include <string.h>

#define HERMES_BUFFER(x) \
    (hermes_hash_api_bstr_t) { (uint8_t *)x, strlen(x) }

// Logging test function
bool test_blake2b_512_function()
{

    hermes_hash_api_bstr_t buf = HERMES_BUFFER("test test");
    uint8_t outlen = 64;
    hermes_hash_api_bstr_t ret;
    hermes_hash_api_errno_t err;

    if (hermes_hash_api_blake2b(&buf, &outlen, &ret, &err))
    {
        // Check if the hash we got returned is the correct size.
        if (ret.len == 64)
        {
            // Constant binary data to compare against
            const unsigned char constantData[] = {
                0x8e, 0x27, 0xb2, 0x48, 0x1d, 0xd1, 0xfe, 0x73,
                0xd5, 0x98, 0x10, 0x4c, 0x03, 0xb1, 0xf6, 0x7d,
                0xa6, 0x07, 0x25, 0xab, 0xb7, 0x3c, 0xf6, 0x6e,
                0x40, 0x01, 0x77, 0xd7, 0x3a, 0xee, 0x01, 0xe7,
                0x4b, 0x93, 0xf5, 0x5a, 0xdd, 0xa2, 0x7b, 0x0a,
                0xd9, 0x2e, 0x22, 0xe2, 0x84, 0xb5, 0xe0, 0xcc,
                0x95, 0xad, 0x81, 0xb0, 0x4b, 0x49, 0x6b, 0xd5,
                0x8c, 0x4a, 0xe6, 0xbc, 0xa5, 0xf5, 0x61, 0x96};

            return memcmp(ret.ptr, constantData, sizeof(constantData)) == 0;
        }
    }
    return false;
}

// Exported Functions from `hermes:integration-test/event`
bool exports_hermes_integration_test_event_test(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret)
{
    ret->status = true;
    switch (test)
    {
    case 0:
        hermes_string_dup(&ret->name, "blake2b-512");
        if (run)
            ret->status = test_blake2b_512_function();
        break;

    default:
        return false;
    }

    return true;
}
