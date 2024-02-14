#include "hermes.h"

int main(int argc, char *argv[]) {
  return 0;
}

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

// Helper Functions

void hermes_tuple2_string_string_free(hermes_tuple2_string_string_t *ptr) {

}

void hermes_list_tuple2_string_string_free(hermes_list_tuple2_string_string_t *ptr) {

}

void hermes_list_string_free(hermes_list_string_t *ptr) {

}

void hermes_option_string_free(hermes_option_string_t *ptr) {

}

void wasi_io_streams_stream_error_free(wasi_io_streams_stream_error_t *ptr) {

}

void hermes_list_u8_free(hermes_list_u8_t *ptr) {

}

void wasi_io_streams_result_list_u8_stream_error_free(wasi_io_streams_result_list_u8_stream_error_t *ptr) {

}

void wasi_io_streams_result_u64_stream_error_free(wasi_io_streams_result_u64_stream_error_t *ptr) {

}

void wasi_io_streams_result_void_stream_error_free(wasi_io_streams_result_void_stream_error_t *ptr) {

}

void wasi_filesystem_types_option_datetime_free(wasi_filesystem_types_option_datetime_t *ptr) {

}

void wasi_filesystem_types_descriptor_stat_free(wasi_filesystem_types_descriptor_stat_t *ptr) {

}

void wasi_filesystem_types_new_timestamp_free(wasi_filesystem_types_new_timestamp_t *ptr) {

}

void wasi_filesystem_types_directory_entry_free(wasi_filesystem_types_directory_entry_t *ptr) {

}

void wasi_filesystem_types_result_own_input_stream_error_code_free(wasi_filesystem_types_result_own_input_stream_error_code_t *ptr) {

}

void wasi_filesystem_types_result_own_output_stream_error_code_free(wasi_filesystem_types_result_own_output_stream_error_code_t *ptr) {

}

void wasi_filesystem_types_result_void_error_code_free(wasi_filesystem_types_result_void_error_code_t *ptr) {

}

void wasi_filesystem_types_result_descriptor_flags_error_code_free(wasi_filesystem_types_result_descriptor_flags_error_code_t *ptr) {

}

void wasi_filesystem_types_result_descriptor_type_error_code_free(wasi_filesystem_types_result_descriptor_type_error_code_t *ptr) {

}

void wasi_filesystem_types_result_tuple2_list_u8_bool_error_code_free(wasi_filesystem_types_result_tuple2_list_u8_bool_error_code_t *ptr) {

}

void wasi_filesystem_types_result_filesize_error_code_free(wasi_filesystem_types_result_filesize_error_code_t *ptr) {

}

void wasi_filesystem_types_result_own_directory_entry_stream_error_code_free(wasi_filesystem_types_result_own_directory_entry_stream_error_code_t *ptr) {

}

void wasi_filesystem_types_result_descriptor_stat_error_code_free(wasi_filesystem_types_result_descriptor_stat_error_code_t *ptr) {

}

void wasi_filesystem_types_result_own_descriptor_error_code_free(wasi_filesystem_types_result_own_descriptor_error_code_t *ptr) {

}

void wasi_filesystem_types_result_string_error_code_free(wasi_filesystem_types_result_string_error_code_t *ptr) {

}

void wasi_filesystem_types_result_metadata_hash_value_error_code_free(wasi_filesystem_types_result_metadata_hash_value_error_code_t *ptr) {

}

void wasi_filesystem_types_option_directory_entry_free(wasi_filesystem_types_option_directory_entry_t *ptr) {

}

void wasi_filesystem_types_result_option_directory_entry_error_code_free(wasi_filesystem_types_result_option_directory_entry_error_code_t *ptr) {

}

void wasi_filesystem_types_option_error_code_free(wasi_filesystem_types_option_error_code_t *ptr) {

}

void wasi_filesystem_preopens_tuple2_own_descriptor_string_free(wasi_filesystem_preopens_tuple2_own_descriptor_string_t *ptr) {

}

void wasi_filesystem_preopens_list_tuple2_own_descriptor_string_free(wasi_filesystem_preopens_list_tuple2_own_descriptor_string_t *ptr) {

}

void wasi_http_types_method_free(wasi_http_types_method_t *ptr) {

}

void wasi_http_types_scheme_free(wasi_http_types_scheme_t *ptr) {

}

void hermes_option_u16_free(hermes_option_u16_t *ptr) {

}

void wasi_http_types_dns_error_payload_free(wasi_http_types_dns_error_payload_t *ptr) {

}

void hermes_option_u8_free(hermes_option_u8_t *ptr) {

}

void wasi_http_types_tls_alert_received_payload_free(wasi_http_types_tls_alert_received_payload_t *ptr) {

}

void hermes_option_u32_free(hermes_option_u32_t *ptr) {

}

void wasi_http_types_field_size_payload_free(wasi_http_types_field_size_payload_t *ptr) {

}

void hermes_option_u64_free(hermes_option_u64_t *ptr) {

}

void wasi_http_types_option_field_size_payload_free(wasi_http_types_option_field_size_payload_t *ptr) {

}

void wasi_http_types_error_code_free(wasi_http_types_error_code_t *ptr) {

}

void wasi_http_types_header_error_free(wasi_http_types_header_error_t *ptr) {

}

void wasi_http_types_field_key_free(wasi_http_types_field_key_t *ptr) {

}

void wasi_http_types_field_value_free(wasi_http_types_field_value_t *ptr) {

}

void wasi_http_types_option_error_code_free(wasi_http_types_option_error_code_t *ptr) {

}

void hermes_tuple2_field_key_field_value_free(hermes_tuple2_field_key_field_value_t *ptr) {

}

void hermes_list_tuple2_field_key_field_value_free(hermes_list_tuple2_field_key_field_value_t *ptr) {

}

void wasi_http_types_result_own_fields_header_error_free(wasi_http_types_result_own_fields_header_error_t *ptr) {

}

void hermes_list_field_value_free(hermes_list_field_value_t *ptr) {

}

void wasi_http_types_result_void_header_error_free(wasi_http_types_result_void_header_error_t *ptr) {

}

void wasi_http_types_option_scheme_free(wasi_http_types_option_scheme_t *ptr) {

}

void wasi_http_types_result_own_incoming_body_void_free(wasi_http_types_result_own_incoming_body_void_t *ptr) {

}

void wasi_http_types_result_own_outgoing_body_void_free(wasi_http_types_result_own_outgoing_body_void_t *ptr) {

}

void wasi_http_types_result_void_void_free(wasi_http_types_result_void_void_t *ptr) {

}

void hermes_option_duration_free(hermes_option_duration_t *ptr) {

}

void wasi_http_types_result_own_outgoing_response_error_code_free(wasi_http_types_result_own_outgoing_response_error_code_t *ptr) {

}

void wasi_http_types_result_own_input_stream_void_free(wasi_http_types_result_own_input_stream_void_t *ptr) {

}

void wasi_http_types_option_own_trailers_free(wasi_http_types_option_own_trailers_t *ptr) {

}

void wasi_http_types_result_option_own_trailers_error_code_free(wasi_http_types_result_option_own_trailers_error_code_t *ptr) {

}

void wasi_http_types_result_result_option_own_trailers_error_code_void_free(wasi_http_types_result_result_option_own_trailers_error_code_void_t *ptr) {

}

void wasi_http_types_option_result_result_option_own_trailers_error_code_void_free(wasi_http_types_option_result_result_option_own_trailers_error_code_void_t *ptr) {

}

void wasi_http_types_result_own_output_stream_void_free(wasi_http_types_result_own_output_stream_void_t *ptr) {

}

void wasi_http_types_result_void_error_code_free(wasi_http_types_result_void_error_code_t *ptr) {

}

void wasi_http_types_result_own_incoming_response_error_code_free(wasi_http_types_result_own_incoming_response_error_code_t *ptr) {

}

void wasi_http_types_result_result_own_incoming_response_error_code_void_free(wasi_http_types_result_result_own_incoming_response_error_code_void_t *ptr) {

}

void wasi_http_types_option_result_result_own_incoming_response_error_code_void_free(wasi_http_types_option_result_result_own_incoming_response_error_code_void_t *ptr) {

}

void wasi_http_outgoing_handler_error_code_free(wasi_http_outgoing_handler_error_code_t *ptr) {

}

void wasi_http_outgoing_handler_option_own_request_options_free(wasi_http_outgoing_handler_option_own_request_options_t *ptr) {

}

void wasi_http_outgoing_handler_result_own_future_incoming_response_error_code_free(wasi_http_outgoing_handler_result_own_future_incoming_response_error_code_t *ptr) {

}

void hermes_binary_api_bstr_free(hermes_binary_api_bstr_t *ptr) {

}

void hermes_cbor_api_bstr_free(hermes_cbor_api_bstr_t *ptr) {

}

void hermes_cbor_api_cbor_free(hermes_cbor_api_cbor_t *ptr) {

}

void hermes_cardano_api_cbor_free(hermes_cardano_api_cbor_t *ptr) {

}

void hermes_cardano_api_cardano_block_free(hermes_cardano_api_cardano_block_t *ptr) {

}

void hermes_cardano_api_cardano_txn_free(hermes_cardano_api_cardano_txn_t *ptr) {

}

void hermes_cardano_api_slot_free(hermes_cardano_api_slot_t *ptr) {

}

void hermes_cardano_api_result_u64_fetch_error_free(hermes_cardano_api_result_u64_fetch_error_t *ptr) {

}

void hermes_cardano_api_result_cardano_block_fetch_error_free(hermes_cardano_api_result_cardano_block_fetch_error_t *ptr) {

}

void hermes_list_cardano_txn_free(hermes_list_cardano_txn_t *ptr) {

}

void hermes_cardano_api_result_void_txn_error_free(hermes_cardano_api_result_void_txn_error_t *ptr) {

}

void hermes_cron_api_cron_event_tag_free(hermes_cron_api_cron_event_tag_t *ptr) {

}

void hermes_cron_api_cron_sched_free(hermes_cron_api_cron_sched_t *ptr) {

}

void hermes_cron_api_cron_tagged_free(hermes_cron_api_cron_tagged_t *ptr) {

}

void hermes_cron_api_cron_component_free(hermes_cron_api_cron_component_t *ptr) {

}

void hermes_cron_api_cron_time_free(hermes_cron_api_cron_time_t *ptr) {

}

void hermes_option_cron_event_tag_free(hermes_option_cron_event_tag_t *ptr) {

}

void hermes_cron_api_tuple2_cron_tagged_bool_free(hermes_cron_api_tuple2_cron_tagged_bool_t *ptr) {

}

void hermes_cron_api_list_tuple2_cron_tagged_bool_free(hermes_cron_api_list_tuple2_cron_tagged_bool_t *ptr) {

}

void hermes_crypto_api_bstr_free(hermes_crypto_api_bstr_t *ptr) {

}

void hermes_option_ed25519_bip32_private_key_free(hermes_option_ed25519_bip32_private_key_t *ptr) {

}

void hermes_hash_api_bstr_free(hermes_hash_api_bstr_t *ptr) {

}

void hermes_option_bstr_free(hermes_option_bstr_t *ptr) {

}

void hermes_hash_api_result_bstr_errno_free(hermes_hash_api_result_bstr_errno_t *ptr) {

}

void hermes_json_api_json_free(hermes_json_api_json_t *ptr) {

}

void hermes_kv_store_api_bstr_free(hermes_kv_store_api_bstr_t *ptr) {

}

void hermes_kv_store_api_cbor_free(hermes_kv_store_api_cbor_t *ptr) {

}

void hermes_kv_store_api_json_free(hermes_kv_store_api_json_t *ptr) {

}

void hermes_kv_store_api_kv_values_free(hermes_kv_store_api_kv_values_t *ptr) {

}

void hermes_kv_store_api_option_kv_values_free(hermes_kv_store_api_option_kv_values_t *ptr) {

}

void hermes_localtime_api_timezone_free(hermes_localtime_api_timezone_t *ptr) {

}

void hermes_localtime_api_localtime_free(hermes_localtime_api_localtime_t *ptr) {

}

void hermes_localtime_api_option_datetime_free(hermes_localtime_api_option_datetime_t *ptr) {

}

void hermes_option_timezone_free(hermes_option_timezone_t *ptr) {

}

void hermes_localtime_api_result_localtime_errno_free(hermes_localtime_api_result_localtime_errno_t *ptr) {

}

void hermes_localtime_api_result_datetime_errno_free(hermes_localtime_api_result_datetime_errno_t *ptr) {

}

void hermes_logging_api_json_free(hermes_logging_api_json_t *ptr) {

}

void hermes_option_json_free(hermes_option_json_t *ptr) {

}

void exports_hermes_cardano_event_on_block_cardano_block_free(exports_hermes_cardano_event_on_block_cardano_block_t *ptr) {

}

void exports_hermes_cardano_event_on_txn_cardano_txn_free(exports_hermes_cardano_event_on_txn_cardano_txn_t *ptr) {

}

void exports_hermes_cron_event_cron_event_tag_free(exports_hermes_cron_event_cron_event_tag_t *ptr) {

}

void exports_hermes_cron_event_cron_tagged_free(exports_hermes_cron_event_cron_tagged_t *ptr) {

}

void exports_hermes_kv_store_event_kv_values_free(exports_hermes_kv_store_event_kv_values_t *ptr) {

}

// Transfers ownership of `s` into the string `ret`
void hermes_string_set(hermes_string_t *ret, char*s) {

}

// Creates a copy of the input nul-terminate string `s` and
// stores it into the component model string `ret`.
void hermes_string_dup(hermes_string_t *ret, const char*s) {

}

// Deallocates the string pointed to by `ret`, deallocating
// the memory behind the string.
void hermes_string_free(hermes_string_t *ret) {

}
