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

#define HERMES_STRING(x) \
  (hermes_string_t) { (uint8_t *)x, strlen(x) }

hermes_crypto_api_borrow_bip32_ed25519_t get_or_add_resource()
{
  hermes_string_t mnemonic_string = HERMES_STRING("prevent company field green slot measure chief hero apple task eagle sunset endorse dress seed");
  hermes_crypto_api_mnemonic_phrase_t mnemonic = {.ptr = &mnemonic_string, .len = 1};

  hermes_string_t passphrase_string = {.ptr = NULL, .len = 0};
  hermes_crypto_api_passphrase_t passphrase = {.ptr = &passphrase_string, .len = 0};

  hermes_crypto_api_own_bip32_ed25519_t resource = hermes_crypto_api_constructor_bip32_ed25519(&mnemonic, NULL);
  hermes_crypto_api_borrow_bip32_ed25519_t borrow_resource = hermes_crypto_api_borrow_bip32_ed25519(resource);
  return borrow_resource;
}

bool generate_mnemonic()
{
  hermes_string_t prefix_data = HERMES_STRING("project");

  hermes_crypto_api_prefix_t prefix = {.ptr = &prefix_data, .len = 1};
  hermes_string_t language = HERMES_STRING("English");

  hermes_crypto_api_mnemonic_phrase_t ret;
  hermes_crypto_api_errno_t err;

  hermes_crypto_api_generate_mnemonic(24, &prefix, &language, &ret, &err);

  char *expected_prefix = "project";
  size_t n = strlen(expected_prefix);

  return ret.ptr != NULL && ret.ptr->ptr != NULL && ret.ptr->len >= n &&
         strncmp((const char *)ret.ptr->ptr, expected_prefix, n) == 0;
}

bool get_pubkey()
{
  hermes_crypto_api_borrow_bip32_ed25519_t borrow_resource = get_or_add_resource();
  hermes_crypto_api_bip32_ed25519_public_key_t ret;
  hermes_crypto_api_method_bip32_ed25519_public_key(borrow_resource, &ret);

  return (ret.f0 == 3986768884739312704ULL) &&
         (ret.f1 == 9782938079688165927ULL) &&
         (ret.f2 == 7977656244723921923ULL) &&
         (ret.f3 == 12587033252467133758ULL);
}

// Exported Functions from `hermes:integration-test/event`
bool exports_hermes_integration_test_event_test(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret)
{
  ret->status = true;
  switch (test)
  {
  case 0:
    hermes_string_dup(&ret->name, "Generate mnemonic");
    if (run)
      ret->status = generate_mnemonic();
    break;
  case 1:
    hermes_string_dup(&ret->name, "BIP32-Ed25519");
    if (run)
      ret->status = get_pubkey();
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

bool exports_hermes_http_gateway_event_reply(exports_hermes_http_gateway_event_bstr_t *body, exports_hermes_http_gateway_event_headers_t *headers, hermes_string_t *path, hermes_string_t *method, exports_hermes_http_gateway_event_http_response_t *ret){
  return false;
};

void exports_hermes_http_request_event_on_http_response(uint64_t *maybe_request_id, hermes_list_u8_t *response) {}