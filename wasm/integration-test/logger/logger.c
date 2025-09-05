#include "bindings_src/hermes.h"
#include <string.h>

#define HERMES_STRING(x) \
    (hermes_string_t) { (uint8_t *)x, strlen(x) }
#define HERMES_JSON_STRING(x) \
    (hermes_json_api_json_t) { (uint8_t *)x, strlen(x) }

// Logging test function
bool test_logging_function()
{
    hermes_string_t file;
    hermes_string_t function;
    uint32_t line;
    uint32_t col;
    hermes_string_t ctx;
    hermes_string_t msg;
    hermes_json_api_json_t data;

    file = HERMES_STRING("filename.c");
    function = HERMES_STRING("main");
    line = 11;
    col = 6;
    ctx = HERMES_STRING("Context");
    msg = HERMES_STRING("Log Message");
    data = HERMES_JSON_STRING("{\"key\":\"value\"}");

    hermes_logging_api_log(2, &file, &function, &line, &col, &ctx, &msg, &data);

    return true;
}

// Exported Functions from `hermes:integration-test/event`
bool exports_hermes_integration_test_event_test(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret)
{
    ret->status = true;
    switch (test)
    {
    case 0:
        hermes_string_dup(&ret->name, "Call Logger");
        if (run)
            ret->status = test_logging_function();
        break;

    default:
        return false;
    }

    return true;
}
