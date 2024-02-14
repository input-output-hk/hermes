# 1. Overview

The `cardano-chain-follower` crate provides functionality to read arbitrary blocks
and follow updates (new blocks and rollbacks) from a Cardano network (e.g. mainnet, preprod).

Currently, the all communication with a Cardano node (remote or local) is done using the
[Node-to-Node protocol](https://docs.cardano.org/explore-cardano/cardano-network/about-the-cardano-network).
A [Mithril snapshot](https://github.com/input-output-hk/mithril) can be configured to be used both when reading blocks
and following chain updates.

The [Pallas](https://github.com/txpipe/pallas) crate is used under the hood to provide
node communication, block parsing and other Cardano chain features.

## 1.1 Chain Follow

The chain follower is capable of receiving chain updates from a Cardano node using the ChainSync miniprotocol.

```kroki-excalidraw
@from_file:architecture/08_concepts/cardano_chain_follower/images/overview.excalidraw
```

### Read pointer

The read pointer points at the location the chain is being read by a client connection.
Although the Cardano node maintains a read pointer for each client, the chain follower manages
its own copy of the read pointer in order to follow the chain even when it's reading data from a Mithril snapshot.
The follower's read pointer gets updated every time it receives a chain update.

### Chain Updates

The chain follower spawns a background task that keeps a Node-to-Node connection to a Cardano node
and continuously receives updates from it and sends them to the follower using a async channel.
A chain update can be either a roll forward (a new block added to the chain) or a rollback.

If any node communication error happens in the background task, this is also sent through the channel.

If the follower has been configured to use a Mithril snapshot, it will generate
synthetic roll forward chain updates for each block until the snapshot's tip is reached.
After that, updates are received from the node.

If any errors occur while reading the block from the Mithril snapshot (e.g. the block is missing from the snapshot, I/O errors)
the background task will fallback to receiving the failed block from the Cardano node.

Below is a simplified flow diagram of the background task's process for producing chain updates.

#### A. Chain update flow diagram

```kroki-excalidraw
@from_file:architecture/08_concepts/cardano_chain_follower/images/simplified-get-update-flow.excalidraw
```

## 1.2 Chain Read

*NOTE: Reading blocks does not affect the follower read pointer.*

When reading a single or a range of arbitrary blocks from the chain the follower initiates a new connection with the configured node
blocks are read using the Blockfetch miniprotocol.
If configured, available blocks are read from the Mithril snapshot as well.

When a block is requested, the follower will try reading the block from the Mithril snapshot
first (if configured) and, only if the block is not found, it'll ask the connected node for the block.

When a range of blocks is requested, the follower will try reading as many blocks as it can from the Mithril snapshot
(if configured) and, if any blocks are not contained in the snapshot, it'll ask the connected node for them.

Below is a simplified flow diagram of the block reading logic.

### A. Single block flow diagram

```kroki-excalidraw
@from_file:architecture/08_concepts/cardano_chain_follower/images/simplified-reader-single-block-flow.excalidraw
```

### B. Block range flow diagram

```kroki-excalidraw
@from_file:architecture/08_concepts/cardano_chain_follower/images/simplified-reader-block-range-flow.excalidraw
```
