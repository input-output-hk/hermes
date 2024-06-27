# Things that need fixing

The following fixes would be nice to have to:

1. Improve sync times.
2. Decrease disk utilization.
3. Eliminate external dependencies.

## Parallel downloading requires external tool

We currently require an external tool `aria2c` to download the Mithril snapshot.
We should have a native version to remove this external tool dependency.

See: <https://ochagavia.nl/blog/download-accelerator-async-rust-edition/>
For a simple version of such we could adapt.

The first version should just replace `Aria2c` and download to a file.

Ideally, we would have an in-memory queue that downloads in parallel, rather than saving to disk.
This would need to use something like a skip-map to re-order the blocks, and a pool of workers to download the next blocks.
It's not trivial, but it would remove the necessity to store the actual snapshot archive on-disk.

It's not possible to download the snapshot archive to ram because it is enormous.

## Zstd decompress and tar extraction optimization

Currently, an async zstd decompress and tar extraction is used.
This is known to be slow, and we are CPU bound doing it.

Change this to run in a Thread outside async and use the zstd library, which links to the C zstd library directly.
And the non async tar extraction library.  

This will speed up extracting files from the archive.

This would be better also if we had synchronous piped downloading as mentioned above.

## Block Decode Optimization

Currently, to enforce and validate chain integrity, we need to decode the blocks all over the place.
Decoding blocks is expensive, and this is wasteful.
As the application will almost certainly require the block to be decoded, it makes sense for it to happen once in a uniform way.  
We would then pass the decoded block to the application saving it the effort of doing it, itself.

We should Decode LIVE blocks once when we receive them from the network,
and then keep the decoded as well as raw block data in memory.

For Immutable blocks, we should decode them ONCE when we read them from disk.

## Immutable Queue Optimization

The Immutable follower reads from disk, inline.
Disk IO is relatively expensive.
Decoding blocks is also expensive, it's better to do that in parallel with an application processing a previous block.

What we should do is have a read ahead queue, where a second task is reading ahead of the application following,
reading the next blocks from disk, and decoding them.

The main follower used by the application then reads from this red ahead queue.
This would help us better utilize disk and CPU resources, which would result in improved sync times.
