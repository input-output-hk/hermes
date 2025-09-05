#include "bindings_src/hermes.h"
#include <string.h>

#define HERMES_STRING(x) \
    (hermes_string_t) { (uint8_t *)x, strlen(x) }

// Localtime test function
bool test_localtime_function()
{
    hermes_localtime_api_localtime_t ret;
    hermes_localtime_api_errno_t err;
    
    if (hermes_localtime_api_get_localtime(NULL, &HERMES_STRING("Europe/London"), &ret, &err)) {
        return true;
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
        hermes_string_dup(&ret->name, "get_localtime");
        if (run)
            ret->status = test_localtime_function();
        break;
    
    default:
        return false;
    }

    return true;
}
