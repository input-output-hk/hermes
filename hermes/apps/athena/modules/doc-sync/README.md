# Doc Sync Module

Thin wrapper for posting documents to IPFS PubSub.
The actual 4-step workflow (file_add, file_pin, pre-publish, pubsub_publish) is executed on the host side for efficiency.

## Usage

```rust
use doc_sync::channel;

let cid = channel::post(document_bytes)?;
```

## Architecture

The doc-sync module provides a simple API for publishing documents to IPFS via PubSub channels.
All heavy operations are delegated to the host-side implementation for performance:

1. **file_add** - Add document to IPFS
2. **file_pin** - Pin the document
3. **pre-publish** - Prepare for PubSub
4. **pubsub_publish** - Publish to the channel

## HTTP Gateway

The module exposes an HTTP endpoint for testing:

```bash
curl -X POST http://localhost:7878/api/doc-sync/post \
  -H "Host: athena.hermes.local" \
  -H "Content-Type: text/plain" \
  -d "Hello, IPFS!"
```
