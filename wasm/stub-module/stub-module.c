#include "bindings_src/hermes.h"

// Exported Functions from `wasi:http/incoming-handler@0.2.0`
void exports_wasi_http_incoming_handler_handle(exports_wasi_http_incoming_handler_own_incoming_request_t request, exports_wasi_http_incoming_handler_own_response_outparam_t response_out) {

}

// Exported Functions from `hermes:cardano/event-on-block`
void exports_hermes_cardano_event_on_block_on_cardano_block(exports_hermes_cardano_event_on_block_borrow_subscription_id_t subscription_id, exports_hermes_cardano_event_on_block_borrow_block_t block)
{
}

// Exported Functions from `hermes:cardano/event-on-immutable-roll-forward`
void exports_hermes_cardano_event_on_immutable_roll_forward_on_cardano_immutable_roll_forward(exports_hermes_cardano_event_on_immutable_roll_forward_borrow_subscription_id_t subscription_id, exports_hermes_cardano_event_on_immutable_roll_forward_borrow_block_t block)
{
}

// Exported Functions from `hermes:cron/event`
bool exports_hermes_cron_event_on_cron(exports_hermes_cron_event_cron_tagged_t *event, bool last) {
  return false;
}


// Exported Functions from `hermes:http-gateway/event`
bool exports_hermes_http_gateway_event_reply(exports_hermes_http_gateway_event_bstr_t *body, exports_hermes_http_gateway_event_headers_t *headers, hermes_string_t *path, hermes_string_t *method, exports_hermes_http_gateway_event_http_gateway_response_t *ret){
  return false;
};


// Exported Functions from `hermes:init/event`
bool exports_hermes_init_event_init(void) {
  return false;
}

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