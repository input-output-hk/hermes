#include "bindings_src/hermes.h"
#include <string.h>

// Monotonic clock test function
bool test_monotonic_now_function()
{
    wasi_clocks_monotonic_clock_instant_t one = wasi_clocks_monotonic_clock_now();
    wasi_clocks_monotonic_clock_instant_t two = wasi_clocks_monotonic_clock_now();
    
    return one <= two;
}

// Exported Functions from `hermes:integration-test/event`
bool exports_hermes_integration_test_event_test(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret)
{
    ret->status = true;
    switch (test)
    {
    case 0:
        hermes_string_dup(&ret->name, "clocks_monotonic_now");
        if (run)
            ret->status = test_monotonic_now_function();
        break;
    
    default:
        return false;
    }

    return true;
}
