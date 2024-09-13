# Overhead Benchmark

The purpose of this benchmark is to measure the overhead that using the `cardano-chain-follower` crate
has on top of the `pallas` crate which is used to implement most of the chain follower features.

## Running

In order to execute the benchmark you need a valid Mithril snapshot to point it to.
It doesn't matter which network the snapshot is from because the benchmark will only read the data from it.

There are 2 modes in which the benchmark can be executed:

| Benchmark name           | Description  |
|--------------------------|--------------|
| pallas                 | When executed with `--bench-name pallas`, the benchmark reads the Mithril snapshot from origin to its tip using only the `pallas` crate mechanisms |
| cardano&#x2011;chain&#x2011;follower | When executed with `--bench-name cardano-chain-follower` it uses the `cardano-chain-follower` crate to follow the chain from origin to the tip of the specified snapshot |

One way of executing the benchmark is as follows:

```sh
cargo run --release -- --bench-name cardano-chain-follower --mithril-snapshot-path PATH_TO_MITHRIL_SNAPSHOT
```

## Earthfile

The Earthfile has targets for building and running the benchmark.
It also contains targets to fetch Mithril snapshots and save them locally.
