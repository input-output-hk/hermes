#include "bindings_src/hermes.h"

// Exported Functions from `hermes:init/event`
//bool exports_hermes_init_event_init(void) {return false;}
bool event_init(void) {
  return hermes_init_event_init();
}

// Exported Functions from `hermes:kv-store/event`
void kv_update(hermes_string_t *key, exports_hermes_kv_store_event_kv_values_t *value) {
  return hermes_kv_store_event_kv_update(key, value);
}

// Exported Functions from `hermes:cron/event`
bool on_cron(exports_hermes_cron_event_cron_tagged_t *event, bool last) {
  return hermes_cron_event_on_cron(event, last);
}

void on_cardano_block(exports_hermes_cardano_event_on_block_borrow_subscription_id_t subscription_id, exports_hermes_cardano_event_on_block_borrow_block_t block) {
  return event_on_block_on_cardano_block(subscription_id, block);
}

void on_cardano_immutable_roll_forward(exports_hermes_cardano_event_on_immutable_roll_forward_borrow_subscription_id_t subscription_id, exports_hermes_cardano_event_on_immutable_roll_forward_borrow_block_t block) {
  return hermes_cardano_event_on_cardano_immutable_roll_forward(subscription_id, block);
}

void on_cardano_immutable_roll_forward(event_borrow_subscription_id_t subscription_id, event_borrow_block_t block) {
  return hermes_cardano_event_on_immutable_roll_forward_on_cardano_immutable_roll_forward(subscription_id, block);
}

// Exported Functions from `wasi:http/incoming-handler@0.2.0`
void handle(exports_wasi_http_incoming_handler_own_incoming_request_t request, exports_wasi_http_incoming_handler_own_response_outparam_t response_out) {
  return wasi_http_incoming_handler_handle(request, response_out);
}

// Exported Functions from `hermes:http-gateway/event`
bool reply(exports_hermes_http_gateway_event_bstr_t *body, exports_hermes_http_gateway_event_headers_t *headers, hermes_string_t *path, hermes_string_t *method, exports_hermes_http_gateway_event_http_gateway_response_t *ret){
  return hermes_http_gateway_event_reply(body, headers, path, method, ret);
};

// Exported Functions from `hermes:ipfs/event`
bool on_topic(exports_hermes_ipfs_event_pubsub_message_t *message) {
  return hermes_ipfs_event_on_topic(message);
}

// Exported Functions from `hermes:integration-test/event`
bool test(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret) {
  return hermes_integration_test_event_test(test, run, ret);
}

bool bench(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret) {
  return hermes_integration_test_event_bench(test, run, ret);
}

void on_http_response(uint64_t *maybe_request_id, hermes_list_u8_t *response) {
  return hermes_http_request_event_on_http_response(maybe_request_id, response);
}