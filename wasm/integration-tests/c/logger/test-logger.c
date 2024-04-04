#include "../bindings_src/hermes.h"
#include <string.h>

const uint32_t N_TEST = 1;
const exports_hermes_integration_test_event_test_result_t TESTS[N_TEST] = {
    {.name = {
         .ptr = (uint8_t *)"Logging 1",
         .len = strlen("Logging 1")},
     .status = true},
};

const uint32_t N_BENCH = 0;
const exports_hermes_integration_test_event_test_result_t BENCHES[N_BENCH] = {
    {}};

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

// Logging test function
bool test_logging_function(uint32_t test)
{
  hermes_string_t file;
  hermes_string_t function;
  uint32_t line;
  uint32_t col;
  hermes_string_t ctx;
  hermes_string_t msg;
  hermes_json_api_json_t data;

  switch (test)
  {
  case 0:
    file = (hermes_string_t){"filename.c", 10};
    function = (hermes_string_t){"main", 4};
    line = 11;
    col = 6;
    ctx = (hermes_string_t){"context", 7};
    msg = (hermes_string_t){"Log message", 11};
    data = (hermes_json_api_json_t){"{\"key\":\"value\"}", 15};
    hermes_logging_api_log(2, &file, &function, &line, &col, &ctx, &msg, &data);
    return true;
  default:
    return false;
  }
}

// Exported Functions from `hermes:integration-test/event`
bool exports_hermes_integration_test_event_test(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret)
{
  if (test < N_TEST)
  {
    hermes_string_dup(&ret->name, TESTS[test].name.ptr);
    if (run)
    {
      ret->status = test_logging_function(test);
    }
    return true;
  }
  return false;
}

bool exports_hermes_integration_test_event_bench(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret)
{
  return false;
}
