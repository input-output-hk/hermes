#include "bindings_src/hermes.h"
#include <string.h>

// Exported Functions from `wasi:http/incoming-handler@0.2.0`
void exports_wasi_http_incoming_handler_handle(exports_wasi_http_incoming_handler_own_incoming_request_t request, exports_wasi_http_incoming_handler_own_response_outparam_t response_out)
{
}

// Exported Functions from `hermes:cardano/event-on-block`
void exports_hermes_cardano_event_on_block_on_cardano_block(exports_hermes_cardano_event_on_block_cardano_blockchain_id_t blockchain, exports_hermes_cardano_event_on_block_cardano_block_t *block, exports_hermes_cardano_event_on_block_block_src_t source)
{
}

// Exported Functions from `hermes:cardano/event-on-txn`
void exports_hermes_cardano_event_on_txn_on_cardano_txn(exports_hermes_cardano_event_on_txn_cardano_blockchain_id_t blockchain, uint64_t slot, uint32_t txn_index, exports_hermes_cardano_event_on_txn_cardano_txn_t *txn)
{
}

// Exported Functions from `hermes:cardano/event-on-rollback`
void exports_hermes_cardano_event_on_rollback_on_cardano_rollback(exports_hermes_cardano_event_on_rollback_cardano_blockchain_id_t blockchain, uint64_t slot)
{
}

// Exported Functions from `hermes:cron/event`
bool exports_hermes_cron_event_on_cron(exports_hermes_cron_event_cron_tagged_t *event, bool last)
{
    return false;
}

// Exported Functions from `hermes:init/event`
bool exports_hermes_init_event_init(void)
{
    return false;
}

// Exported Functions from `hermes:kv-store/event`
void exports_hermes_kv_store_event_kv_update(hermes_string_t *key, exports_hermes_kv_store_event_kv_values_t *value)
{
}

#define HERMES_STRING(x) \
    (hermes_string_t) { (uint8_t *)x, strlen(x) }
#define HERMES_JSON_STRING(x) \
    (hermes_json_api_json_t) { (uint8_t *)x, strlen(x) }

// Wall clock test function
bool test_wall_now_function()
{
    wasi_clocks_wall_clock_datetime_t ret;

    wasi_clocks_wall_clock_now(&ret);

    return true;
}

// Monotonic clock test function
bool test_monotonic_now_function()
{
    wasi_clocks_monotonic_clock_instant_t ret = wasi_clocks_monotonic_clock_now();
    
    return true;
}

// Exported Functions from `hermes:integration-test/event`
bool exports_hermes_integration_test_event_test(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret)
{
    ret->status = true;
    switch (test)
    {
    case 0:
        hermes_string_dup(&ret->name, "clocks_wall_now");
        if (run)
            ret->status = test_wall_now_function();
        break;
    case 1:
        hermes_string_dup(&ret->name, "clocks_monotonic_now");
        if (run)
            ret->status = test_monotonic_now_function();
        break;
    
    default:
        return false;
    }

    return true;
}

bool exports_hermes_integration_test_event_bench(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret)
{
    return false;
}
