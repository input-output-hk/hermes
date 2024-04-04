#include "bindings_src/hermes.h"
#include <stdlib.h>
#include <string.h>

// Function to convert a hexadecimal character to its integer value
int hexCharToInt(char c) {
  if (c >= '0' && c <= '9')
    return c - '0';
  if (c >= 'A' && c <= 'F')
    return c - 'A' + 10;
  if (c >= 'a' && c <= 'f')
    return c - 'a' + 10;
  return -1; // Invalid character
}

uint8_t *hex2bin(const char *hex_str) {
  size_t hex_str_len = strlen(hex_str);
  if (hex_str_len % 2 != 0) {
    // Odd number of characters in the hexadecimal string
    return NULL;
  }

  size_t bytes_needed = strlen(hex_str) / 2;

  uint8_t *bytes_arr = (uint8_t *)malloc(bytes_needed);
  if (bytes_arr == NULL) {
    return NULL;
  }
  
  for (size_t i = 0; i < bytes_needed; i++) {
    int high_nibble = hexCharToInt(hex_str[i * 2]);
    int low_nibble = hexCharToInt(hex_str[i * 2 + 1]);
    if (high_nibble == -1 || low_nibble == -1) {
      // Invalid hexadecimal character
      return NULL;
    }
    bytes_arr[i] = (uint8_t)((high_nibble << 4) | low_nibble);
  }

  return bytes_arr;
}

hermes_hash_api_bstr_t bstr_t_from(uint8_t *str) {
  return (hermes_hash_api_bstr_t){
    .ptr = str,
    .len = strlen(str)
  };
}