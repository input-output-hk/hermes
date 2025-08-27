//! Filesystem host implementation for WASM runtime.

use std::io::{Seek, SeekFrom};

use super::state::{get_state, Descriptor};
use crate::{
    hdf5::Path,
    runtime_context::HermesRuntimeContext,
    runtime_extensions::{
        bindings::wasi::{
            filesystem::{
                self,
                types::{
                    Advice, Descriptor as WasiDescriptor, DescriptorFlags, DescriptorStat,
                    DescriptorType, DirectoryEntry, DirectoryEntryStream, Error, ErrorCode,
                    Filesize, MetadataHashValue, NewTimestamp, OpenFlags, PathFlags,
                },
            },
            io::streams::{InputStream, OutputStream},
        },
        wasi::io::streams::{get_input_streams_state, get_output_streams_state},
    },
};

impl filesystem::types::HostDescriptor for HermesRuntimeContext {
    /// Return a stream for reading from a file, if available.
    ///
    /// May fail with an error-code describing why the file cannot be read.
    ///
    /// Multiple read, write, and append streams may be active on the same open
    /// file and they do not interfere with each other.
    ///
    /// Note: This allows using `read-stream`, which is similar to `read` in POSIX.
    fn read_via_stream(
        &mut self,
        res: wasmtime::component::Resource<WasiDescriptor>,
        offset: Filesize,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<InputStream>, ErrorCode>> {
        let mut fs_app_state = get_state().get_app_state(self.app_name())?;
        let Ok(descriptor) = fs_app_state.get_object(&res) else {
            return Ok(Err(ErrorCode::BadDescriptor));
        };
        let mut file = match &*descriptor {
            Descriptor::File(f) => f.clone(),
            Descriptor::Dir(_) => return Ok(Err(ErrorCode::IsDirectory)),
        };
        file.seek(SeekFrom::Start(offset))?;

        let input_streams_app_state = get_input_streams_state().get_app_state(self.app_name())?;
        Ok(Ok(input_streams_app_state.create_resource(Box::new(file))))
    }

    /// Return a stream for writing to a file, if available.
    ///
    /// May fail with an error-code describing why the file cannot be written.
    ///
    /// Note: This allows using `write-stream`, which is similar to `write` in
    /// POSIX.
    fn write_via_stream(
        &mut self,
        res: wasmtime::component::Resource<WasiDescriptor>,
        offset: Filesize,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<OutputStream>, ErrorCode>> {
        let mut fs_app_state = get_state().get_app_state(self.app_name())?;
        let Ok(descriptor) = fs_app_state.get_object(&res) else {
            return Ok(Err(ErrorCode::BadDescriptor));
        };
        let mut file = match &*descriptor {
            Descriptor::File(f) => f.clone(),
            Descriptor::Dir(_) => return Ok(Err(ErrorCode::IsDirectory)),
        };
        file.seek(SeekFrom::Start(offset))?;

        let output_streams_app_state = get_output_streams_state().get_app_state(self.app_name())?;
        Ok(Ok(output_streams_app_state.create_resource(Box::new(file))))
    }

    /// Return a stream for appending to a file, if available.
    ///
    /// May fail with an error-code describing why the file cannot be appended.
    ///
    /// Note: This allows using `write-stream`, which is similar to `write` with
    /// `O_APPEND` in in POSIX.
    fn append_via_stream(
        &mut self,
        res: wasmtime::component::Resource<WasiDescriptor>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<OutputStream>, ErrorCode>> {
        let mut fs_app_state = get_state().get_app_state(self.app_name())?;
        let Ok(descriptor) = fs_app_state.get_object(&res) else {
            return Ok(Err(ErrorCode::BadDescriptor));
        };
        let mut file = match &*descriptor {
            Descriptor::File(f) => f.clone(),
            Descriptor::Dir(_) => return Ok(Err(ErrorCode::IsDirectory)),
        };
        file.seek(SeekFrom::End(0))?;

        let output_streams_app_state = get_output_streams_state().get_app_state(self.app_name())?;
        Ok(Ok(output_streams_app_state.create_resource(Box::new(file))))
    }

    /// Provide file advisory information on a descriptor.
    ///
    /// This is similar to `posix_fadvise` in POSIX.
    fn advise(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _offset: Filesize,
        _length: Filesize,
        _advice: Advice,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Synchronize the data of a file to disk.
    ///
    /// This function succeeds with no effect if the file descriptor is not
    /// opened for writing.
    ///
    /// Note: This is similar to `fdatasync` in POSIX.
    fn sync_data(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Get flags associated with a descriptor.
    ///
    /// Note: This returns similar flags to `fcntl(fd, F_GETFL)` in POSIX.
    ///
    /// Note: This returns the value that was the `fs_flags` value returned
    /// from `fdstat_get` in earlier versions of WASI.
    fn get_flags(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
    ) -> wasmtime::Result<Result<DescriptorFlags, ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Get the dynamic type of a descriptor.
    ///
    /// Note: This returns the same value as the `type` field of the `fd-stat`
    /// returned by `stat`, `stat-at` and similar.
    ///
    /// Note: This returns similar flags to the `st_mode & S_IFMT` value provided
    /// by `fstat` in POSIX.
    ///
    /// Note: This returns the value that was the `fs_filetype` value returned
    /// from `fdstat_get` in earlier versions of WASI.
    fn get_type(
        &mut self,
        res: wasmtime::component::Resource<WasiDescriptor>,
    ) -> wasmtime::Result<Result<DescriptorType, ErrorCode>> {
        let mut app_state = get_state().get_app_state(self.app_name())?;
        let Ok(descriptor) = app_state.get_object(&res) else {
            return Ok(Err(ErrorCode::BadDescriptor));
        };

        let dt = match &*descriptor {
            Descriptor::File(_) => DescriptorType::RegularFile,
            Descriptor::Dir(_) => DescriptorType::Directory,
        };

        Ok(Ok(dt))
    }

    /// Adjust the size of an open file. If this increases the file\'s size, the
    /// extra bytes are filled with zeros.
    ///
    /// Note: This was called `fd_filestat_set_size` in earlier versions of WASI.
    fn set_size(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _size: Filesize,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Adjust the timestamps of an open file or directory.
    ///
    /// Note: This is similar to `futimens` in POSIX.
    ///
    /// Note: This was called `fd_filestat_set_times` in earlier versions of WASI.
    fn set_times(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _data_access_timestamp: NewTimestamp,
        _data_modification_timestamp: NewTimestamp,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Read from a descriptor, without using and updating the descriptor\'s offset.
    ///
    /// This function returns a list of bytes containing the data that was
    /// read, along with a bool which, when true, indicates that the end of the
    /// file was reached. The returned list will contain up to `length` bytes; it
    /// may return fewer than requested, if the end of the file is reached or
    /// if the I/O operation is interrupted.
    ///
    /// In the future, this may change to return a `stream<u8, error-code>`.
    ///
    /// Note: This is similar to `pread` in POSIX.
    fn read(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _length: Filesize,
        _offset: Filesize,
    ) -> wasmtime::Result<Result<(Vec<u8>, bool), ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Write to a descriptor, without using and updating the descriptor\'s offset.
    ///
    /// It is valid to write past the end of a file; the file is extended to the
    /// extent of the write, with bytes between the previous end and the start of
    /// the write set to zero.
    ///
    /// In the future, this may change to take a `stream<u8, error-code>`.
    ///
    /// Note: This is similar to `pwrite` in POSIX.
    fn write(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _buffer: Vec<u8>,
        _offset: Filesize,
    ) -> wasmtime::Result<Result<Filesize, ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Read directory entries from a directory.
    ///
    /// On filesystems where directories contain entries referring to themselves
    /// and their parents, often named `.` and `..` respectively, these entries
    /// are omitted.
    ///
    /// This always returns a new stream which starts at the beginning of the
    /// directory. Multiple streams may be active on the same directory, and they
    /// do not interfere with each other.
    fn read_directory(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<DirectoryEntryStream>, ErrorCode>>
    {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Synchronize the data and metadata of a file to disk.
    ///
    /// This function succeeds with no effect if the file descriptor is not
    /// opened for writing.
    ///
    /// Note: This is similar to `fsync` in POSIX.
    fn sync(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Create a directory.
    ///
    /// Note: This is similar to `mkdirat` in POSIX.
    fn create_directory_at(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Return the attributes of an open file or directory.
    ///
    /// Note: This is similar to `fstat` in POSIX, except that it does not return
    /// device and inode information. For testing whether two descriptors refer to
    /// the same underlying filesystem object, use `is-same-object`. To obtain
    /// additional data that can be used do determine whether a file has been
    /// modified, use `metadata-hash`.
    ///
    /// Note: This was called `fd_filestat_get` in earlier versions of WASI.
    fn stat(
        &mut self,
        res: wasmtime::component::Resource<WasiDescriptor>,
    ) -> wasmtime::Result<Result<DescriptorStat, ErrorCode>> {
        let mut app_state = get_state().get_app_state(self.app_name())?;
        let Ok(descriptor) = app_state.get_object(&res) else {
            return Ok(Err(ErrorCode::BadDescriptor));
        };

        let f = match &*descriptor {
            Descriptor::File(f) => f,
            Descriptor::Dir(_) => {
                #[allow(clippy::todo)]
                {
                    todo!()
                }
            },
        };

        let Ok(size) = f
            .size()
            .and_then(|size| TryInto::<u64>::try_into(size).map_err(|e| anyhow::anyhow!(e)))
        else {
            return Ok(Err(ErrorCode::Io));
        };

        Ok(Ok(DescriptorStat {
            type_: DescriptorType::RegularFile,
            link_count: 0,
            size,
            data_access_timestamp: None,
            data_modification_timestamp: None,
            status_change_timestamp: None,
        }))
    }

    /// Return the attributes of a file or directory.
    ///
    /// Note: This is similar to `fstatat` in POSIX, except that it does not
    /// return device and inode information. See the `stat` description for a
    /// discussion of alternatives.
    ///
    /// Note: This was called `path_filestat_get` in earlier versions of WASI.
    fn stat_at(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _path_flags: PathFlags,
        _path: String,
    ) -> wasmtime::Result<Result<DescriptorStat, ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Adjust the timestamps of a file or directory.
    ///
    /// Note: This is similar to `utimensat` in POSIX.
    ///
    /// Note: This was called `path_filestat_set_times` in earlier versions of
    /// WASI.
    fn set_times_at(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _path_flags: PathFlags,
        _path: String,
        _data_access_timestamp: NewTimestamp,
        _data_modification_timestamp: NewTimestamp,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Create a hard link.
    ///
    /// Note: This is similar to `linkat` in POSIX.
    fn link_at(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _old_path_flags: PathFlags,
        _old_path: String,
        _new_descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _new_path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Open a file or directory.
    ///
    /// The returned descriptor is not guaranteed to be the lowest-numbered
    /// descriptor not currently open/ it is randomized to prevent applications
    /// from depending on making assumptions about indexes, since this is
    /// error-prone in multi-threaded contexts. The returned descriptor is
    /// guaranteed to be less than 2**31.
    ///
    /// If `flags` contains `descriptor-flags::mutate-directory`, and the base
    /// descriptor doesn't have `descriptor-flags::mutate-directory` set,
    /// `open-at` fails with `error-code::read-only`.
    ///
    /// If `flags` contains `write` or `mutate-directory`, or `open-flags`
    /// contains `truncate` or `create`, and the base descriptor doesn't have
    /// `descriptor-flags::mutate-directory` set, `open-at` fails with
    /// `error-code::read-only`.
    ///
    /// Note: This is similar to `openat` in POSIX.
    fn open_at(
        &mut self,
        res: wasmtime::component::Resource<WasiDescriptor>,
        _path_flags: PathFlags,
        path: String,
        open_flags: OpenFlags,
        _flags: DescriptorFlags,
    ) -> wasmtime::Result<Result<wasmtime::component::Resource<WasiDescriptor>, ErrorCode>> {
        let mut app_state = get_state().get_app_state(self.app_name())?;
        let Ok(descriptor) = app_state.get_object(&res) else {
            return Ok(Err(ErrorCode::BadDescriptor));
        };
        let dir = match &*descriptor {
            Descriptor::Dir(dir) => dir,
            Descriptor::File(_) => return Ok(Err(ErrorCode::NotDirectory)),
        };

        let create = open_flags.contains(OpenFlags::CREATE);
        let exclusive = open_flags.contains(OpenFlags::EXCLUSIVE);
        let f = match dir.get_file(Path::from_str(&path)) {
            Ok(f) => {
                if create && exclusive {
                    return Ok(Err(ErrorCode::Exist));
                }

                f
            },
            Err(_) => {
                if create {
                    if let Ok(f) = dir.create_file(Path::from_str(&path)) {
                        f
                    } else {
                        return Ok(Err(ErrorCode::Io));
                    }
                } else {
                    return Ok(Err(ErrorCode::NoEntry));
                }
            },
        };

        let f = if open_flags.contains(OpenFlags::TRUNCATE) {
            if dir.remove_file(Path::from_str(&path)).is_err() {
                return Ok(Err(ErrorCode::Io));
            }

            match dir.create_file(Path::from_str(&path)) {
                Ok(f) => f,
                Err(_) => return Ok(Err(ErrorCode::Io)),
            }
        } else {
            f
        };
        drop(descriptor);
        Ok(Ok(app_state.create_resource(Descriptor::File(f))))
    }

    /// Read the contents of a symbolic link.
    ///
    /// If the contents contain an absolute or rooted path in the underlying
    /// filesystem, this function fails with `error-code::not-permitted`.
    ///
    /// Note: This is similar to `readlinkat` in POSIX.
    fn readlink_at(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _path: String,
    ) -> wasmtime::Result<Result<String, ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Remove a directory.
    ///
    /// Return `error-code::not-empty` if the directory is not empty.
    ///
    /// Note: This is similar to `unlinkat(fd, path, AT_REMOVEDIR)` in POSIX.
    fn remove_directory_at(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Rename a filesystem object.
    ///
    /// Note: This is similar to `renameat` in POSIX.
    fn rename_at(
        &mut self,
        _old_descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _old_path: String,
        _new_descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _new_path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Create a symbolic link (also known as a "symlink").
    ///
    /// If `old-path` starts with `/`, the function fails with
    /// `error-code::not-permitted`.
    ///
    /// Note: This is similar to `symlinkat` in POSIX.
    fn symlink_at(
        &mut self,
        _old_descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _old_path: String,
        _new_path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Unlink a filesystem object that is not a directory.
    ///
    /// Return `error-code::is-directory` if the path refers to a directory.
    /// Note: This is similar to `unlinkat(fd, path, 0)` in POSIX.
    fn unlink_file_at(
        &mut self,
        res: wasmtime::component::Resource<WasiDescriptor>,
        path: String,
    ) -> wasmtime::Result<Result<(), ErrorCode>> {
        let mut app_state = get_state().get_app_state(self.app_name())?;
        let Ok(descriptor) = app_state.get_object(&res) else {
            return Ok(Err(ErrorCode::BadDescriptor));
        };

        match &*descriptor {
            Descriptor::Dir(dir) => {
                let path: Path = path.into();

                if dir.get_file(path.clone()).is_err() {
                    return Ok(Err(ErrorCode::NoEntry));
                }

                if dir.remove_file(path).is_err() {
                    Ok(Err(ErrorCode::Io))
                } else {
                    Ok(Ok(()))
                }
            },
            Descriptor::File(_) => Ok(Err(ErrorCode::NotDirectory)),
        }
    }

    /// Test whether two descriptors refer to the same filesystem object.
    ///
    /// In POSIX, this corresponds to testing whether the two descriptors have the
    /// same device (`st_dev`) and inode (`st_ino` or `d_ino`) numbers.
    /// wasi-filesystem does not expose device and inode numbers, so this function
    /// may be used instead.
    fn is_same_object(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _other: wasmtime::component::Resource<WasiDescriptor>,
    ) -> wasmtime::Result<bool> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    /// Return a hash of the metadata associated with a filesystem object referred
    /// to by a descriptor.
    ///
    /// This returns a hash of the last-modification timestamp and file size, and
    /// may also include the inode number, device number, birth timestamp, and
    /// other metadata fields that may change when the file is modified or
    /// replaced. It may also include a secret value chosen by the
    /// implementation and not otherwise exposed.
    ///
    /// Implementations are encouraged to provide the following properties:
    ///
    /// - If the file is not modified or replaced, the computed hash value should usually
    ///   not change.
    /// - If the object is modified or replaced, the computed hash value should usually
    ///   change.
    /// - The inputs to the hash should not be easily computable from the computed hash.
    ///
    /// However, none of these is required.
    fn metadata_hash(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
    ) -> wasmtime::Result<Result<MetadataHashValue, ErrorCode>> {
        // TODO: Compute the actual hash
        Ok(Ok(MetadataHashValue { lower: 0, upper: 0 }))
    }

    /// Return a hash of the metadata associated with a filesystem object referred
    /// to by a directory descriptor and a relative path.
    ///
    /// This performs the same hash computation as `metadata-hash`.
    fn metadata_hash_at(
        &mut self,
        _descriptor: wasmtime::component::Resource<WasiDescriptor>,
        _path_flags: PathFlags,
        _path: String,
    ) -> wasmtime::Result<Result<MetadataHashValue, ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    fn drop(
        &mut self,
        res: wasmtime::component::Resource<WasiDescriptor>,
    ) -> wasmtime::Result<()> {
        let app_state = get_state().get_app_state(self.app_name())?;
        app_state.delete_resource(res)?;
        Ok(())
    }
}

impl filesystem::types::HostDirectoryEntryStream for HermesRuntimeContext {
    /// Read a single directory entry from a `directory-entry-stream`.
    fn read_directory_entry(
        &mut self,
        _dir: wasmtime::component::Resource<DirectoryEntryStream>,
    ) -> wasmtime::Result<Result<Option<DirectoryEntry>, ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }

    fn drop(
        &mut self,
        _rep: wasmtime::component::Resource<DirectoryEntryStream>,
    ) -> wasmtime::Result<()> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }
}

impl filesystem::types::Host for HermesRuntimeContext {
    /// Attempts to extract a filesystem-related `error-code` from the stream
    /// `error` provided.
    ///
    /// Stream operations which return `stream-error::last-operation-failed`
    /// have a payload with more information about the operation that failed.
    /// This payload can be passed through to this function to see if there\'s
    /// filesystem-related information about the error to return.
    ///
    /// Note that this function is fallible because not all stream-related
    /// errors are filesystem-related errors.
    fn filesystem_error_code(
        &mut self,
        _err: wasmtime::component::Resource<Error>,
    ) -> wasmtime::Result<Option<ErrorCode>> {
        #[allow(clippy::todo)]
        {
            todo!()
        }
    }
}

impl filesystem::preopens::Host for HermesRuntimeContext {
    /// Return the set of preopened directories, and their path.
    fn get_directories(
        &mut self
    ) -> wasmtime::Result<Vec<(wasmtime::component::Resource<WasiDescriptor>, String)>> {
        let vfs_root = self.vfs().root().clone();
        let app_state = get_state().get_app_state(self.app_name())?;
        let res = app_state.create_resource(Descriptor::Dir(vfs_root));
        Ok(vec![(res, "/".to_string())])
    }
}
