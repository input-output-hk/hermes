#include "../bindings_src/hermes.h"
#include <string.h>
#include <time.h>

const uint32_t N_TEST = 4;
const exports_hermes_integration_test_event_test_result_t TESTS[N_TEST] = {
    {.name = {
         .ptr = (uint8_t *)"Crypto generate mnemonic 1",
         .len = strlen("Crypto generate mnemonic 1")},
     .status = true},
    {.name = {.ptr = (uint8_t *)"Crypto get pub key 2", .len = strlen("Crypto get pub key 2")}, .status = true},
    {.name = {.ptr = (uint8_t *)"Crypto sign and check sig 3", .len = strlen("Crypto sign and check sig 3")}, .status = true},
    {.name = {.ptr = (uint8_t *)"Crypto derive 4", .len = strlen("Crypto derive 4")}, .status = true},
};

const uint32_t N_BENCH = 0;
const exports_hermes_integration_test_event_test_result_t BENCHES[N_BENCH] = {
    {}
};

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

hermes_crypto_api_borrow_bip32_ed25519_t get_or_add_resource()
{
  const char *mnemonic_str = "prevent company field green slot measure chief hero apple task eagle sunset endorse dress seed";

  hermes_string_t mnemonic_string = {.ptr = (uint8_t *)mnemonic_str, .len = strlen(mnemonic_str)};
  hermes_crypto_api_mnemonic_phrase_t mnemonic = {.ptr = &mnemonic_string, .len = 1};
  hermes_string_t passphrase_string = {.ptr = NULL, .len = 0};
  hermes_crypto_api_passphrase_t passphrase = {.ptr = &passphrase_string, .len = 0};
  hermes_crypto_api_own_bip32_ed25519_t resource = hermes_crypto_api_constructor_bip32_ed25519(&mnemonic, NULL);
  hermes_crypto_api_borrow_bip32_ed25519_t borrow_resource = hermes_crypto_api_borrow_bip32_ed25519(resource);
  return borrow_resource;
}

// Cryptography test function
bool test_crypto_function(uint32_t test)
{
  switch (test)
  {
  case 0:
  {
    hermes_string_t prefix_data = {.ptr = (uint8_t *)"project", .len = 7};
    hermes_crypto_api_prefix_t prefix = {.ptr = &prefix_data, .len = 1};
    hermes_string_t language = {.ptr = (uint8_t *)"English", .len = 7};
    hermes_crypto_api_mnemonic_phrase_t ret;
    hermes_crypto_api_errno_t err;

    hermes_crypto_api_generate_mnemonic(24, &prefix, &language, &ret, &err);

    char *expected_prefix = "project";
    size_t n = strlen(expected_prefix);

    return ret.ptr != NULL && ret.ptr->ptr != NULL && ret.ptr->len >= n &&
           strncmp(ret.ptr->ptr, expected_prefix, n) == 0;
  }
  case 1:
  {
    hermes_crypto_api_borrow_bip32_ed25519_t borrow_resource = get_or_add_resource();
    hermes_crypto_api_bip32_ed25519_public_key_t ret;
    hermes_crypto_api_method_bip32_ed25519_public_key(borrow_resource, &ret);

    return (ret.f0 == 3986768884739312704) &&
           (ret.f1 == 9782938079688165927ULL) &&
           (ret.f2 == 7977656244723921923) &&
           (ret.f3 == 12587033252467133758ULL);
  }
  case 2:
  {
    hermes_crypto_api_borrow_bip32_ed25519_t borrow_resource = get_or_add_resource();

    hermes_string_t data_string = {.ptr = (uint8_t *)"test", .len = 4};
    hermes_crypto_api_bstr_t sign_data = {.ptr = &data_string, .len = 1};
    hermes_crypto_api_bip32_ed25519_signature_t ret;

    hermes_crypto_api_method_bip32_ed25519_sign_data(borrow_resource, &sign_data, &ret);
    return hermes_crypto_api_method_bip32_ed25519_check_sig(borrow_resource, &sign_data, &ret);
  }
  case 3:
  {
    hermes_crypto_api_borrow_bip32_ed25519_t borrow_resource = get_or_add_resource();

    hermes_string_t path_string = {.ptr = (uint8_t *)"m/1852'/1815'/0'/2/0", .len = strlen("m/1852'/1815'/0'/2/0")};
    hermes_crypto_api_own_bip32_ed25519_t new_resource = hermes_crypto_api_method_bip32_ed25519_derive(borrow_resource, &path_string);
    return new_resource.__handle == 2;
  }
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
      ret->status = test_crypto_function(test);
    }
    return true;
  }
  return false;
}

bool exports_hermes_integration_test_event_bench(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret)
{
  return false;
}