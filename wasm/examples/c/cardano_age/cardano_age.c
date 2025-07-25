#include "bindings_src/hermes.h"
#include <string.h>
#include <stdio.h>

/********** Example **********/

#define HERMES_STRING(x) \
    (hermes_string_t) { (uint8_t *)x, strlen(x) }

void log_cardano_age(double days)
{
  hermes_string_t file;
  hermes_string_t msg;
  char msg_buffer[64];
  int msg_len;
  
  file = HERMES_STRING("cardano_age.c");
  msg_len = snprintf(msg_buffer, sizeof msg_buffer, "Cardano is live for %f days!", days);

  // Discarding encoding errors.
  if (msg_len < 0) {
    msg_len = 0;
  }

  msg = (hermes_string_t) { (uint8_t *)msg_buffer, msg_len };

  hermes_logging_api_log(2, &file, NULL, NULL, NULL, NULL, &msg, NULL);
}

// Exported Functions from `hermes:init/event`
bool exports_hermes_init_event_init(void)
{
  const uint64_t cardano_launch_seconds = 1506246291;
  const uint64_t seconds_in_a_day = 24 * 60 * 60;

  uint64_t elapsed_seconds;
  double elapsed_days;
  wasi_clocks_wall_clock_datetime_t now;
  
  wasi_clocks_wall_clock_now(&now);

  elapsed_seconds = (uint64_t)now.seconds - cardano_launch_seconds;
  // Saturating subtraction.
  elapsed_seconds &= -(elapsed_seconds <= cardano_launch_seconds);

  elapsed_days = (double)elapsed_seconds / seconds_in_a_day;
  log_cardano_age(elapsed_days);

  hermes_init_api_done(0);

  return true;
}

/********** Stub **********/

// Exported Functions from `wasi:http/incoming-handler@0.2.0`
void exports_wasi_http_incoming_handler_handle(exports_wasi_http_incoming_handler_own_incoming_request_t request, exports_wasi_http_incoming_handler_own_response_outparam_t response_out) {

}

// Exported Functions from `hermes:cardano/event-on-block`
void exports_hermes_cardano_event_on_block_on_cardano_block(exports_hermes_cardano_event_on_block_cardano_blockchain_id_t blockchain, exports_hermes_cardano_event_on_block_cardano_block_t *block, exports_hermes_cardano_event_on_block_block_src_t source) {

}

// Exported Functions from `hermes:cardano/event-on-txn`
void exports_hermes_cardano_event_on_txn_on_cardano_txn(exports_hermes_cardano_event_on_txn_cardano_blockchain_id_t blockchain, uint64_t slot, uint32_t txn_index, exports_hermes_cardano_event_on_txn_cardano_txn_t *txn) {

}

// Exported Functions from `hermes:cardano/event-on-rollback`
void exports_hermes_cardano_event_on_rollback_on_cardano_rollback(exports_hermes_cardano_event_on_rollback_cardano_blockchain_id_t blockchain, uint64_t slot) {

}

// Exported Functions from `hermes:cron/event`
bool exports_hermes_cron_event_on_cron(exports_hermes_cron_event_cron_tagged_t *event, bool last) {
  return false;
}

// Exported Functions from `hermes:http-gateway/event`
bool exports_hermes_http_gateway_event_reply(exports_hermes_http_gateway_event_bstr_t *body, exports_hermes_http_gateway_event_headers_t *headers, hermes_string_t *path, hermes_string_t *method, exports_hermes_http_gateway_event_http_response_t *ret){
  return false;
};

// Exported Functions from `hermes:ipfs/event`
bool exports_hermes_ipfs_event_on_topic(exports_hermes_ipfs_event_pubsub_message_t *message) {
  return false;
}

// Exported Functions from `hermes:kv-store/event`
void exports_hermes_kv_store_event_kv_update(hermes_string_t *key, exports_hermes_kv_store_event_kv_values_t *value) {

}

// Exported Functions from `hermes:integration-test/event`
bool exports_hermes_integration_test_event_test(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret) {
  return false;
}

bool exports_hermes_integration_test_event_bench(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret) {
  return false;
}

void exports_hermes_http_request_event_on_http_response(uint64_t *maybe_request_id, hermes_list_u8_t *response) {
  
}
