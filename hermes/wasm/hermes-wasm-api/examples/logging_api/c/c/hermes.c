// Generated by `wit-bindgen` 0.4.0. DO NOT EDIT!
#include "hermes.h"


__attribute__((import_module("logging"), import_name("log")))
void __wasm_import_logging_log(int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t);

__attribute__((weak, export_name("cabi_realloc")))
void *cabi_realloc(void *ptr, size_t old_size, size_t align, size_t new_size) {
  if (new_size == 0) return (void*) align;
  void *ret = realloc(ptr, new_size);
  if (!ret) abort();
  return ret;
}

// Helper Functions

void types_json_free(types_json_t *ptr) {
  hermes_string_free(ptr);
}

void types_cbor_free(types_cbor_t *ptr) {
  if (ptr->len > 0) {
    free(ptr->ptr);
  }
}

void logging_json_free(logging_json_t *ptr) {
  types_json_free(ptr);
}

void hermes_string_set(hermes_string_t *ret, const char*s) {
  ret->ptr = (char*) s;
  ret->len = strlen(s);
}

void hermes_string_dup(hermes_string_t *ret, const char*s) {
  ret->len = strlen(s);
  ret->ptr = cabi_realloc(NULL, 0, 1, ret->len * 1);
  memcpy(ret->ptr, s, ret->len * 1);
}

void hermes_string_free(hermes_string_t *ret) {
  if (ret->len > 0) {
    free(ret->ptr);
  }
  ret->ptr = NULL;
  ret->len = 0;
}

// Component Adapters

void logging_log(logging_level_t level, hermes_string_t *file, hermes_string_t *fn, uint32_t line, hermes_string_t *msg, logging_json_t *data) {
  __wasm_import_logging_log((int32_t) level, (int32_t) (*file).ptr, (int32_t) (*file).len, (int32_t) (*fn).ptr, (int32_t) (*fn).len, (int32_t) (line), (int32_t) (*msg).ptr, (int32_t) (*msg).len, (int32_t) (*data).ptr, (int32_t) (*data).len);
}

__attribute__((export_name("init#init")))
int32_t __wasm_export_init_init(void) {
  bool ret = init_init();
  return ret;
}

extern void __component_type_object_force_link_hermes(void);
void __component_type_object_force_link_hermes_public_use_in_this_compilation_unit(void) {
  __component_type_object_force_link_hermes();
}
