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

// Exported Functions from `hermes:ipfs/event`
bool exports_hermes_ipfs_event_on_topic(exports_hermes_ipfs_event_pubsub_message_t *message) {
  return false;
}

// Exported Functions from `hermes:kv-store/event`
void exports_hermes_kv_store_event_kv_update(hermes_string_t *key, exports_hermes_kv_store_event_kv_values_t *value)
{
}

#define HERMES_BUFFER(x) \
    (hermes_hash_api_bstr_t) { (uint8_t *)x, strlen(x) }

#define HERMES_STRING(x) \
    (hermes_string_t) { (uint8_t *)x, strlen(x) }
#define HERMES_JSON_STRING(x) \
    (hermes_json_api_json_t) { (uint8_t *)x, strlen(x) }

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

bool exports_hermes_integration_test_event_bench(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret)
{
    return false;
}

bool exports_hermes_http_gateway_event_reply(exports_hermes_http_gateway_event_bstr_t *body, exports_hermes_http_gateway_event_headers_t *headers, hermes_string_t *path, hermes_string_t *method, exports_hermes_http_gateway_event_http_gateway_response_t *ret){
  return false;
};

void exports_hermes_http_request_event_on_http_response(uint64_t *maybe_request_id, hermes_list_u8_t *response) {}