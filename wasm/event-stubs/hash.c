#include <stdint.h>

#include <stddef.h>

typedef struct {
  uint8_t * data;
  size_t length;
}
Bstr;

typedef enum {
  KeyTooBig,
  HashTooBig,
}
Errno;

const char * errno_name(Errno errno) {
  switch (errno) {
  case KeyTooBig:
    return "key-too-big";
  case HashTooBig:
    return "hash-too-big";
  default:
    return "unknown-error";
  }
}

const char * errno_message(Errno errno) {
  switch (errno) {
  case HashTooBig:
    return "The key is larger than supported by the hash function.";
  default:
    return "";
  }
}

typedef struct {
  int code;
  const char * name;
  const char * message;
}
Errno_Debug;

Errno_Debug errno_debug(Errno errno) {
  return (Errno_Debug) {
    errno,
    errno_name(errno),
    errno_message(errno)
  };
}

typedef struct {
  Bstr( * blake2s)(Bstr buf, uint8_t outlen, Bstr key);
  Bstr( * blake2b)(Bstr buf, uint8_t outlen, Bstr key);
  Bstr( * blake3)(Bstr buf, uint8_t outlen, Bstr key);
}
HashAPI;

Bstr blake2s(Bstr buf, uint8_t outlen, Bstr key) {
  Bstr result;
  // Implement the BLAKE2s hashing function here
  return result;
}

Bstr blake2b(Bstr buf, uint8_t outlen, Bstr key) {
  Bstr result;
  // Implement the BLAKE2b hashing function here
  return result;
}

Bstr blake3(Bstr buf, uint8_t outlen, Bstr key) {
  Bstr result;
  // Implement the BLAKE3 hashing function here
  return result;
}

int main() {
  HashAPI api = {
    .blake2s = blake2s,
    .blake2b = blake2b,
    .blake3 = blake3,
  };

  // Use the API functions here
  return 0;
}
