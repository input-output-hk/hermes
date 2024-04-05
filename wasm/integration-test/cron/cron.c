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

const char *tag_str = "Example Tag";
const char *when_str = "* * * * *";

hermes_cron_api_cron_tagged_t example_cron_tagged()
{
    hermes_cron_api_cron_sched_t when = {.ptr = (uint8_t *)when_str, .len = strlen(when_str)};
    hermes_cron_api_cron_event_tag_t tag = {.ptr = (uint8_t *)tag_str, .len = strlen(tag_str)};

    return (hermes_cron_api_cron_tagged_t){ when, tag };
}

bool add_crontab()
{
    hermes_cron_api_cron_tagged_t entry = example_cron_tagged();
    bool retrigger = true;

    return hermes_cron_api_add(&entry, retrigger);
}

bool delay_crontab()
{
    hermes_cron_api_cron_event_tag_t tag = {.ptr = (uint8_t *)tag_str, .len = strlen(tag_str)};
    hermes_cron_api_instant_t duration = 2000000000;
    return hermes_cron_api_delay(duration, &tag);
}

bool list_crontabs()
{
    hermes_cron_api_cron_event_tag_t maybe_tag = {.ptr = NULL, .len = 0};
    hermes_cron_api_list_tuple2_cron_tagged_bool_t ret;
    hermes_cron_api_ls(&maybe_tag, &ret);
    return ret.ptr != NULL && ret.len == 0;
}

bool remove_crontab()
{
    hermes_cron_api_cron_tagged_t entry = example_cron_tagged();
    return !hermes_cron_api_rm(&entry);
}

bool make_cron()
{
    hermes_cron_api_cron_component_t all = { .tag = HERMES_CRON_API_CRON_COMPONENT_ALL };
    hermes_cron_api_cron_time_t ctime = { .ptr = &all, .len = 1 };
    hermes_cron_api_cron_sched_t ret;
    hermes_cron_api_mkcron(&ctime, &ctime, &ctime, &ctime, &ctime, &ret);

    char *expected_sched = "* * * * *";
    size_t n = strlen(expected_sched);
    return ret.ptr != NULL && ret.len == n;
}

// Exported Functions from `hermes:integration-test/event`
bool exports_hermes_integration_test_event_test(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret)
{
  ret->status = true;
  switch (test)
  {
  case 0:
    hermes_string_dup(&ret->name, "Add Crontab");
    if (run)
      ret->status = add_crontab();
    break;
  case 1:
    hermes_string_dup(&ret->name, "Delay Crontab");
    if (run)
      ret->status = delay_crontab();
    break;
  case 2:
    hermes_string_dup(&ret->name, "List Crontabs");
    if (run)
      ret->status = list_crontabs();
    break;
  case 3:
    hermes_string_dup(&ret->name, "Remove Crontab");
    if (run)
      ret->status = remove_crontab();
    break;
  case 4:
    hermes_string_dup(&ret->name, "Make Cron Entry");
    if (run)
      ret->status = make_cron();
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
