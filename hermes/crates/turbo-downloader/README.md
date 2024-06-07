# Turbo-Downloader

This library is used to download, and optionally, decompress and extract tar archives at once.
So basically, it does the same as command:

```sh
curl http://archive.tar.lz4 | lz4 -dc | tar -xv -C output_dir
```

It uses multiple download connections to the upstream host, which improves download performance.
This is especially important when retrieving files from rate limited hosts, such as Google or AWS Cloud storage.

Additionally, it records the progress of the download so that it can be logged or reported.
The implementation is non-blocking, based on threads (no async IO used)

It spawns 3 or more threads per download, one or more for downloading,
one for decompressing and one for extracting files from the tar archive (writing files also).
It uses chunks, when chunks are filled with data they are gathered and
when the next chunk is full they are processed by decompression/file extraction logic.

Files are written to disk atomically, and if a file on-disk is the same as one in the archive, it is not replaced.
This is intentional behavior.
It allows archives to be extracted without data race conditions if another process is reading files in the same directory.

No unsafe code used.

Currently, it supports:

* Normal files that are not archives (or not compressed).
* `archive.tar.lz4`
* `archive.tar.gz`
* `archive.tar.bz2`
* `archive.tar.zstd`
* `single_file.lz4`
* `single_file.gz`
* `single_file.bz2`
* `single_file.zstd`

As base mode of operation, it uses PARTIAL_CONTENT (206) HTTP status code.
This allows the download to continue operation when a connection is lost.
The downloader will wait patiently until a chunk is available again.

Turbo-Downloader also supports normal GET (200) download when the server does not support PARTIAL_CONTENT.
However, in this case any network timeout/disconnection will lead to an unrecoverable error.

There is no option of restarting download after an unrecoverable error or killed process.
