//! Host - WASI - Filesystem implementations
//!
#![allow(unused_variables)]

use crate::runtime::extensions::{
    wasi::{
        filesystem::{
            self,
            types::{
                Advice, Descriptor, DescriptorFlags, DescriptorStat, DescriptorType,
                DirectoryEntry, DirectoryEntryStream, Error, ErrorCode, Filesize,
                MetadataHashValue, NewTimestamp, OpenFlags, PathFlags,
            },
        },
        io::streams::{InputStream, OutputStream},
    },
    HermesState, NewState,
};

/// WASI State
pub(crate) struct State {}

impl NewState for State {
    fn new(ctx: &crate::wasm::context::Context) -> Self {
        Self {}
    }
}

impl filesystem::types::HostDescriptor for HermesState {
    #[doc = " Return a stream for reading from a file, if available."]
    #[doc = " "]
    #[doc = " May fail with an error-code describing why the file cannot be read."]
    #[doc = " "]
    #[doc = " Multiple read, write, and append streams may be active on the same open"]
    #[doc = " file and they do not interfere with each other."]
    #[doc = " "]
    #[doc = " Note: This allows using `read-stream`, which is similar to `read` in POSIX."]
    fn read_via_stream(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, offset: Filesize,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<InputStream>, ErrorCode>> {
        todo!()
    }

    #[doc = " Return a stream for writing to a file, if available."]
    #[doc = " "]
    #[doc = " May fail with an error-code describing why the file cannot be written."]
    #[doc = " "]
    #[doc = " Note: This allows using `write-stream`, which is similar to `write` in"]
    #[doc = " POSIX."]
    fn write_via_stream(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, offset: Filesize,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<OutputStream>, ErrorCode>> {
        todo!()
    }

    #[doc = " Return a stream for appending to a file, if available."]
    #[doc = " "]
    #[doc = " May fail with an error-code describing why the file cannot be appended."]
    #[doc = " "]
    #[doc = " Note: This allows using `write-stream`, which is similar to `write` with"]
    #[doc = " `O_APPEND` in in POSIX."]
    fn append_via_stream(
        &mut self, self_: wasmtime::component::Resource<Descriptor>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<OutputStream>, ErrorCode>> {
        todo!()
    }

    #[doc = " Provide file advisory information on a descriptor."]
    #[doc = " "]
    #[doc = " This is similar to `posix_fadvise` in POSIX."]
    fn advise(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, offset: Filesize,
        length: Filesize, advice: Advice,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        todo!()
    }

    #[doc = " Synchronize the data of a file to disk."]
    #[doc = " "]
    #[doc = " This function succeeds with no effect if the file descriptor is not"]
    #[doc = " opened for writing."]
    #[doc = " "]
    #[doc = " Note: This is similar to `fdatasync` in POSIX."]
    fn sync_data(
        &mut self, self_: wasmtime::component::Resource<Descriptor>,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        todo!()
    }

    #[doc = " Get flags associated with a descriptor."]
    #[doc = " "]
    #[doc = " Note: This returns similar flags to `fcntl(fd, F_GETFL)` in POSIX."]
    #[doc = " "]
    #[doc = " Note: This returns the value that was the `fs_flags` value returned"]
    #[doc = " from `fdstat_get` in earlier versions of WASI."]
    fn get_flags(
        &mut self, self_: wasmtime::component::Resource<Descriptor>,
    ) -> wasmtime::Result<Result<DescriptorFlags, ErrorCode>> {
        todo!()
    }

    #[doc = " Get the dynamic type of a descriptor."]
    #[doc = " "]
    #[doc = " Note: This returns the same value as the `type` field of the `fd-stat`"]
    #[doc = " returned by `stat`, `stat-at` and similar."]
    #[doc = " "]
    #[doc = " Note: This returns similar flags to the `st_mode & S_IFMT` value provided"]
    #[doc = " by `fstat` in POSIX."]
    #[doc = " "]
    #[doc = " Note: This returns the value that was the `fs_filetype` value returned"]
    #[doc = " from `fdstat_get` in earlier versions of WASI."]
    fn get_type(
        &mut self, self_: wasmtime::component::Resource<Descriptor>,
    ) -> wasmtime::Result<Result<DescriptorType, ErrorCode>> {
        todo!()
    }

    #[doc = " Adjust the size of an open file. If this increases the file\\'s size, the"]
    #[doc = " extra bytes are filled with zeros."]
    #[doc = " "]
    #[doc = " Note: This was called `fd_filestat_set_size` in earlier versions of WASI."]
    fn set_size(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, size: Filesize,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        todo!()
    }

    #[doc = " Adjust the timestamps of an open file or directory."]
    #[doc = " "]
    #[doc = " Note: This is similar to `futimens` in POSIX."]
    #[doc = " "]
    #[doc = " Note: This was called `fd_filestat_set_times` in earlier versions of WASI."]
    fn set_times(
        &mut self, self_: wasmtime::component::Resource<Descriptor>,
        data_access_timestamp: NewTimestamp, data_modification_timestamp: NewTimestamp,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        todo!()
    }

    #[doc = " Read from a descriptor, without using and updating the descriptor\\'s offset."]
    #[doc = " "]
    #[doc = " This function returns a list of bytes containing the data that was"]
    #[doc = " read, along with a bool which, when true, indicates that the end of the"]
    #[doc = " file was reached. The returned list will contain up to `length` bytes; it"]
    #[doc = " may return fewer than requested, if the end of the file is reached or"]
    #[doc = " if the I/O operation is interrupted."]
    #[doc = " "]
    #[doc = " In the future, this may change to return a `stream<u8, error-code>`."]
    #[doc = " "]
    #[doc = " Note: This is similar to `pread` in POSIX."]
    fn read(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, length: Filesize,
        offset: Filesize,
    ) -> wasmtime::Result<Result<(Vec<u8>, bool), ErrorCode>> {
        todo!()
    }

    #[doc = " Write to a descriptor, without using and updating the descriptor\\'s offset."]
    #[doc = " "]
    #[doc = " It is valid to write past the end of a file; the file is extended to the"]
    #[doc = " extent of the write, with bytes between the previous end and the start of"]
    #[doc = " the write set to zero."]
    #[doc = " "]
    #[doc = " In the future, this may change to take a `stream<u8, error-code>`."]
    #[doc = " "]
    #[doc = " Note: This is similar to `pwrite` in POSIX."]
    fn write(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, buffer: Vec<u8>,
        offset: Filesize,
    ) -> wasmtime::Result<Result<Filesize, ErrorCode>> {
        todo!()
    }

    #[doc = " Read directory entries from a directory."]
    #[doc = " "]
    #[doc = " On filesystems where directories contain entries referring to themselves"]
    #[doc = " and their parents, often named `.` and `..` respectively, these entries"]
    #[doc = " are omitted."]
    #[doc = " "]
    #[doc = " This always returns a new stream which starts at the beginning of the"]
    #[doc = " directory. Multiple streams may be active on the same directory, and they"]
    #[doc = " do not interfere with each other."]
    fn read_directory(
        &mut self, self_: wasmtime::component::Resource<Descriptor>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<DirectoryEntryStream>, ErrorCode>>
    {
        todo!()
    }

    #[doc = " Synchronize the data and metadata of a file to disk."]
    #[doc = " "]
    #[doc = " This function succeeds with no effect if the file descriptor is not"]
    #[doc = " opened for writing."]
    #[doc = " "]
    #[doc = " Note: This is similar to `fsync` in POSIX."]
    fn sync(
        &mut self, self_: wasmtime::component::Resource<Descriptor>,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        todo!()
    }

    #[doc = " Create a directory."]
    #[doc = " "]
    #[doc = " Note: This is similar to `mkdirat` in POSIX."]
    fn create_directory_at(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        todo!()
    }

    #[doc = " Return the attributes of an open file or directory."]
    #[doc = " "]
    #[doc = " Note: This is similar to `fstat` in POSIX, except that it does not return"]
    #[doc = " device and inode information. For testing whether two descriptors refer to"]
    #[doc = " the same underlying filesystem object, use `is-same-object`. To obtain"]
    #[doc = " additional data that can be used do determine whether a file has been"]
    #[doc = " modified, use `metadata-hash`."]
    #[doc = " "]
    #[doc = " Note: This was called `fd_filestat_get` in earlier versions of WASI."]
    fn stat(
        &mut self, self_: wasmtime::component::Resource<Descriptor>,
    ) -> wasmtime::Result<Result<DescriptorStat, ErrorCode>> {
        todo!()
    }

    #[doc = " Return the attributes of a file or directory."]
    #[doc = " "]
    #[doc = " Note: This is similar to `fstatat` in POSIX, except that it does not"]
    #[doc = " return device and inode information. See the `stat` description for a"]
    #[doc = " discussion of alternatives."]
    #[doc = " "]
    #[doc = " Note: This was called `path_filestat_get` in earlier versions of WASI."]
    fn stat_at(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, path_flags: PathFlags,
        path: String,
    ) -> wasmtime::Result<Result<DescriptorStat, ErrorCode>> {
        todo!()
    }

    #[doc = " Adjust the timestamps of a file or directory."]
    #[doc = " "]
    #[doc = " Note: This is similar to `utimensat` in POSIX."]
    #[doc = " "]
    #[doc = " Note: This was called `path_filestat_set_times` in earlier versions of"]
    #[doc = " WASI."]
    fn set_times_at(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, path_flags: PathFlags,
        path: String, data_access_timestamp: NewTimestamp,
        data_modification_timestamp: NewTimestamp,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        todo!()
    }

    #[doc = " Create a hard link."]
    #[doc = " "]
    #[doc = " Note: This is similar to `linkat` in POSIX."]
    fn link_at(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, old_path_flags: PathFlags,
        old_path: String, new_descriptor: wasmtime::component::Resource<Descriptor>,
        new_path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        todo!()
    }

    #[doc = " Open a file or directory."]
    #[doc = " "]
    #[doc = " The returned descriptor is not guaranteed to be the lowest-numbered"]
    #[doc = " descriptor not currently open/ it is randomized to prevent applications"]
    #[doc = " from depending on making assumptions about indexes, since this is"]
    #[doc = " error-prone in multi-threaded contexts. The returned descriptor is"]
    #[doc = " guaranteed to be less than 2**31."]
    #[doc = " "]
    #[doc = " If `flags` contains `descriptor-flags::mutate-directory`, and the base"]
    #[doc = " descriptor doesn't have `descriptor-flags::mutate-directory` set,"]
    #[doc = " `open-at` fails with `error-code::read-only`."]
    #[doc = " "]
    #[doc = " If `flags` contains `write` or `mutate-directory`, or `open-flags`"]
    #[doc = " contains `truncate` or `create`, and the base descriptor doesn't have"]
    #[doc = " `descriptor-flags::mutate-directory` set, `open-at` fails with"]
    #[doc = " `error-code::read-only`."]
    #[doc = " "]
    #[doc = " Note: This is similar to `openat` in POSIX."]
    fn open_at(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, path_flags: PathFlags,
        path: String, open_flags: OpenFlags, flags: DescriptorFlags,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<Descriptor>, ErrorCode>> {
        todo!()
    }

    #[doc = " Read the contents of a symbolic link."]
    #[doc = " "]
    #[doc = " If the contents contain an absolute or rooted path in the underlying"]
    #[doc = " filesystem, this function fails with `error-code::not-permitted`."]
    #[doc = " "]
    #[doc = " Note: This is similar to `readlinkat` in POSIX."]
    fn readlink_at(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, path: String,
    ) -> wasmtime::Result<Result<String, ErrorCode>> {
        todo!()
    }

    #[doc = " Remove a directory."]
    #[doc = " "]
    #[doc = " Return `error-code::not-empty` if the directory is not empty."]
    #[doc = " "]
    #[doc = " Note: This is similar to `unlinkat(fd, path, AT_REMOVEDIR)` in POSIX."]
    fn remove_directory_at(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        todo!()
    }

    #[doc = " Rename a filesystem object."]
    #[doc = " "]
    #[doc = " Note: This is similar to `renameat` in POSIX."]
    fn rename_at(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, old_path: String,
        new_descriptor: wasmtime::component::Resource<Descriptor>, new_path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        todo!()
    }

    #[doc = " Create a symbolic link (also known as a \"symlink\")."]
    #[doc = " "]
    #[doc = " If `old-path` starts with `/`, the function fails with"]
    #[doc = " `error-code::not-permitted`."]
    #[doc = " "]
    #[doc = " Note: This is similar to `symlinkat` in POSIX."]
    fn symlink_at(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, old_path: String,
        new_path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        todo!()
    }

    #[doc = " Unlink a filesystem object that is not a directory."]
    #[doc = " "]
    #[doc = " Return `error-code::is-directory` if the path refers to a directory."]
    #[doc = " Note: This is similar to `unlinkat(fd, path, 0)` in POSIX."]
    fn unlink_file_at(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        todo!()
    }

    #[doc = " Test whether two descriptors refer to the same filesystem object."]
    #[doc = " "]
    #[doc = " In POSIX, this corresponds to testing whether the two descriptors have the"]
    #[doc = " same device (`st_dev`) and inode (`st_ino` or `d_ino`) numbers."]
    #[doc = " wasi-filesystem does not expose device and inode numbers, so this function"]
    #[doc = " may be used instead."]
    fn is_same_object(
        &mut self, self_: wasmtime::component::Resource<Descriptor>,
        other: wasmtime::component::Resource<Descriptor>,
    ) -> wasmtime::Result<bool> {
        todo!()
    }

    #[doc = " Return a hash of the metadata associated with a filesystem object referred"]
    #[doc = " to by a descriptor."]
    #[doc = " "]
    #[doc = " This returns a hash of the last-modification timestamp and file size, and"]
    #[doc = " may also include the inode number, device number, birth timestamp, and"]
    #[doc = " other metadata fields that may change when the file is modified or"]
    #[doc = " replaced. It may also include a secret value chosen by the"]
    #[doc = " implementation and not otherwise exposed."]
    #[doc = " "]
    #[doc = " Implementations are encouraged to provide the following properties:"]
    #[doc = " "]
    #[doc = " - If the file is not modified or replaced, the computed hash value should"]
    #[doc = " usually not change."]
    #[doc = " - If the object is modified or replaced, the computed hash value should"]
    #[doc = " usually change."]
    #[doc = " - The inputs to the hash should not be easily computable from the"]
    #[doc = " computed hash."]
    #[doc = " "]
    #[doc = " However, none of these is required."]
    fn metadata_hash(
        &mut self, self_: wasmtime::component::Resource<Descriptor>,
    ) -> wasmtime::Result<Result<MetadataHashValue, ErrorCode>> {
        todo!()
    }

    #[doc = " Return a hash of the metadata associated with a filesystem object referred"]
    #[doc = " to by a directory descriptor and a relative path."]
    #[doc = " "]
    #[doc = " This performs the same hash computation as `metadata-hash`."]
    fn metadata_hash_at(
        &mut self, self_: wasmtime::component::Resource<Descriptor>, path_flags: PathFlags,
        path: String,
    ) -> wasmtime::Result<Result<MetadataHashValue, ErrorCode>> {
        todo!()
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Descriptor>) -> wasmtime::Result<()> {
        todo!()
    }
}

impl filesystem::types::HostDirectoryEntryStream for HermesState {
    #[doc = " Read a single directory entry from a `directory-entry-stream`."]
    fn read_directory_entry(
        &mut self, self_: wasmtime::component::Resource<DirectoryEntryStream>,
    ) -> wasmtime::Result<Result<Option<DirectoryEntry>, ErrorCode>> {
        todo!()
    }

    fn drop(
        &mut self, rep: wasmtime::component::Resource<DirectoryEntryStream>,
    ) -> wasmtime::Result<()> {
        todo!()
    }
}

impl filesystem::types::Host for HermesState {
    #[doc = " Attempts to extract a filesystem-related `error-code` from the stream"]
    #[doc = " `error` provided."]
    #[doc = " "]
    #[doc = " Stream operations which return `stream-error::last-operation-failed`"]
    #[doc = " have a payload with more information about the operation that failed."]
    #[doc = " This payload can be passed through to this function to see if there\\'s"]
    #[doc = " filesystem-related information about the error to return."]
    #[doc = " "]
    #[doc = " Note that this function is fallible because not all stream-related"]
    #[doc = " errors are filesystem-related errors."]
    fn filesystem_error_code(
        &mut self, err: wasmtime::component::Resource<Error>,
    ) -> wasmtime::Result<Option<ErrorCode>> {
        todo!()
    }
}

impl filesystem::preopens::Host for HermesState {
    #[doc = " Return the set of preopened directories, and their path."]
    fn get_directories(
        &mut self,
    ) -> wasmtime::Result<Vec<(wasmtime::component::Resource<Descriptor>, String)>> {
        todo!()
    }
}
