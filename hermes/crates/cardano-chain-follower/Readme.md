# Things that need fixing

## OpenSSL Library usage

The mithril crate force includes openssl through its dependency on reqwest.
We need to make a patch for the mithril crate to take choices for the tls library to be used that maps 1:1 to reqwest.
