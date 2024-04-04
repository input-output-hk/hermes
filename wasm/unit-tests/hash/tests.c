#include "bindings_src/hermes.h"
#include "helpers.c"
#include <string.h>

#define N_TEST 4

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

// Exported Functions from `hermes:init/event`
bool exports_hermes_init_event_init(void) {
  return false;
}

// Exported Functions from `hermes:kv-store/event`
void exports_hermes_kv_store_event_kv_update(hermes_string_t *key, exports_hermes_kv_store_event_kv_values_t *value) {

}

// Exported Functions from `hermes:integration-test/event`
bool exports_hermes_integration_test_event_test(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret) {
  const char *tests[N_TEST] = {
    "blake2b_512",
    "blake2b_256",
    "blake2bmac_512",
    "blake2bmac_hash_too_big_err"
  };

  if (test < N_TEST) {
    hermes_string_dup(&ret->name, tests[test]);
    ret->status = false;

    if (!run) {
      return true;
    }
  } else {
    return false;
  }

  switch (test) {
    // blake2smac
    case 0: {
      hermes_hash_api_bstr_t buf = {
        .ptr = (uint8_t *)"test test",
        .len = strlen("test test")
      };
      uint8_t outlen = 64;

      hermes_hash_api_bstr_t local_ret = {NULL, 0};
      hermes_hash_api_errno_t *local_err = NULL;
      bool success = hermes_hash_api_blake2b(&buf, &outlen, &local_ret, local_err);

      if (success && local_ret.ptr != NULL) {
        int res = strcmp(
          local_ret.ptr,
          hex2bin("8e27b2481dd1fe73d598104c03b1f67da60725abb73cf66e400177d73aee01e74b93f55adda27b0ad92e22e284b5e0cc95ad81b04b496bd58c4ae6bca5f56196")
        );

        ret->status = (res == 0);
      }

      return true;
    }
    case 1: {
      ret->status = false;
      return true;
    }
    // blake2bmac
    case 2: {
      // bool hermes_hash_api_blake2bmac(hermes_hash_api_bstr_t *buf, uint8_t *maybe_outlen, hermes_hash_api_bstr_t *key, hermes_hash_api_bstr_t *maybe_salt, hermes_hash_api_bstr_t *maybe_personal, hermes_hash_api_bstr_t *ret, hermes_hash_api_errno_t *err);
      ret->status = false;
      return true;
    }
    case 3: {
      // bool hermes_hash_api_blake2bmac(hermes_hash_api_bstr_t *buf, uint8_t *maybe_outlen, hermes_hash_api_bstr_t *key, hermes_hash_api_bstr_t *maybe_salt, hermes_hash_api_bstr_t *maybe_personal, hermes_hash_api_bstr_t *ret, hermes_hash_api_errno_t *err);
      ret->status = false;
      return true;
    }
    default: {
      return false;
    }
  }
}

bool exports_hermes_integration_test_event_bench(uint32_t test, bool run, exports_hermes_integration_test_event_test_result_t *ret) {
  return false;
}
