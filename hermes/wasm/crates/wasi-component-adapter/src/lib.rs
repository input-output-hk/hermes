// The proxy world has no filesystem which most of this file is concerned with,
// so disable many warnings to avoid having to contort code too much for the
// proxy world.
#![cfg_attr(
    feature = "proxy",
    allow(
        unused_mut,
        unused_variables,
        dead_code,
        unused_imports,
        unreachable_code
    )
)]

use crate::bindings::wasi::clocks::{monotonic_clock, wall_clock};
use crate::bindings::wasi::io::poll;
use crate::bindings::wasi::io::streams;
use crate::bindings::wasi::random::random;
use core::cell::OnceCell;
use core::cell::{Cell, RefCell, RefMut, UnsafeCell};
use core::cmp::min;
use core::ffi::c_void;
use core::hint::black_box;
use core::mem::{self, align_of, forget, size_of, ManuallyDrop, MaybeUninit};
use core::ops::{Deref, DerefMut};
use core::ptr::{self, null_mut};
use core::slice;
use poll::Pollable;
use wasi::*;

#[cfg(not(feature = "proxy"))]
use crate::bindings::wasi::filesystem::types as filesystem;

#[cfg(any(
    all(feature = "command", feature = "reactor"),
    all(feature = "reactor", feature = "proxy"),
    all(feature = "command", feature = "proxy"),
))]
compile_error!(
    "only one of the `command`, `reactor` or `proxy` features may be selected at a time"
);

#[macro_use]
mod macros;

mod descriptors;
use crate::descriptors::{Descriptor, Descriptors, StreamType, Streams};

pub mod bindings {
    #[cfg(feature = "command")]
    wit_bindgen::generate!({
        path: "../wasi/wit",
        world: "wasi:cli/command",
        std_feature,
        raw_strings,
        // Automatically generated bindings for these functions will allocate
        // Vecs, which in turn pulls in the panic machinery from std, which
        // creates vtables that end up in the wasm elem section, which we
        // can't support in these special core-wasm adapters.
        // Instead, we manually define the bindings for these functions in
        // terms of raw pointers.
        skip: ["run", "get-environment", "poll"],
    });

    #[cfg(feature = "reactor")]
    wit_bindgen::generate!({
        path: "../wasi/wit",
        world: "wasi:cli/imports",
        std_feature,
        raw_strings,
        // Automatically generated bindings for these functions will allocate
        // Vecs, which in turn pulls in the panic machinery from std, which
        // creates vtables that end up in the wasm elem section, which we
        // can't support in these special core-wasm adapters.
        // Instead, we manually define the bindings for these functions in
        // terms of raw pointers.
        skip: ["get-environment", "poll"],
    });

    #[cfg(feature = "proxy")]
    wit_bindgen::generate!({
        path: "./crates/wasi/wit",
        inline: r#"
            package wasmtime:adapter;

            world adapter {
                import wasi:clocks/wall-clock@0.2.0-rc-2023-11-10;
                import wasi:clocks/monotonic-clock@0.2.0-rc-2023-11-10;
                import wasi:random/random@0.2.0-rc-2023-11-10;
                import wasi:cli/stdout@0.2.0-rc-2023-12-05;
                import wasi:cli/stderr@0.2.0-rc-2023-12-05;
                import wasi:cli/stdin@0.2.0-rc-2023-12-05;
            }
        "#,
        std_feature,
        raw_strings,
        skip: ["poll"],
    });
}

#[export_name = "wasi:cli/run@0.2.0-rc-2023-12-05#run"]
#[cfg(feature = "command")]
pub unsafe extern "C" fn run() -> u32 {
    #[link(wasm_import_module = "__main_module__")]
    extern "C" {
        fn _start();
    }
    _start();
    0
}

#[cfg(feature = "proxy")]
macro_rules! cfg_filesystem_available {
    ($($t:tt)*) => {
        wasi::ERRNO_NOTSUP
    };
}
#[cfg(not(feature = "proxy"))]
macro_rules! cfg_filesystem_available {
    ($($t:tt)*) => ($($t)*);
}

// The unwrap/expect methods in std pull panic when they fail, which pulls
// in unwinding machinery that we can't use in the adapter. Instead, use this
// extension trait to get postfixed upwrap on Option and Result.
trait TrappingUnwrap<T> {
    fn trapping_unwrap(self) -> T;
}

impl<T> TrappingUnwrap<T> for Option<T> {
    fn trapping_unwrap(self) -> T {
        match self {
            Some(t) => t,
            None => unreachable!(),
        }
    }
}

impl<T, E> TrappingUnwrap<T> for Result<T, E> {
    fn trapping_unwrap(self) -> T {
        match self {
            Ok(t) => t,
            Err(_) => unreachable!(),
        }
    }
}

/// Allocate a file descriptor which will generate an `ERRNO_BADF` if passed to
/// any WASI Preview 1 function implemented by this adapter.
///
/// This is intended for use by `wasi-libc` during its incremental transition
/// from WASI Preview 1 to Preview 2.  It will use this function to reserve
/// descriptors for its own use, valid only for use with libc functions.
#[no_mangle]
pub unsafe extern "C" fn adapter_open_badfd(fd: *mut u32) -> Errno {
    State::with(|state| {
        *fd = state.descriptors_mut().open(Descriptor::Bad)?;
        Ok(())
    })
}

/// Close a descriptor previously opened using `adapter_open_badfd`.
#[no_mangle]
pub unsafe extern "C" fn adapter_close_badfd(fd: u32) -> Errno {
    State::with(|state| state.descriptors_mut().close(fd))
}

#[no_mangle]
pub unsafe extern "C" fn reset_adapter_state() {
    let state = get_state_ptr();
    if !state.is_null() {
        State::init(state)
    }
}

#[no_mangle]
pub unsafe extern "C" fn cabi_import_realloc(
    old_ptr: *mut u8,
    old_size: usize,
    align: usize,
    new_size: usize,
) -> *mut u8 {
    if !old_ptr.is_null() || old_size != 0 {
        unreachable!();
    }
    let mut ptr = null_mut::<u8>();
    State::with(|state| {
        ptr = state.import_alloc.alloc(align, new_size);
        Ok(())
    });
    ptr
}

/// Bump-allocated memory arena. This is a singleton - the
/// memory will be sized according to `bump_arena_size()`.
pub struct BumpArena {
    data: MaybeUninit<[u8; bump_arena_size()]>,
    position: Cell<usize>,
}

impl BumpArena {
    fn new() -> Self {
        BumpArena {
            data: MaybeUninit::uninit(),
            position: Cell::new(0),
        }
    }
    fn alloc(&self, align: usize, size: usize) -> *mut u8 {
        let start = self.data.as_ptr() as usize;
        let next = start + self.position.get();
        let alloc = align_to(next, align);
        let offset = alloc - start;
        if offset + size > bump_arena_size() {
            unreachable!("out of memory");
        }
        self.position.set(offset + size);
        alloc as *mut u8
    }
}
fn align_to(ptr: usize, align: usize) -> usize {
    (ptr + (align - 1)) & !(align - 1)
}

// Invariant: buffer not-null and arena is-some are never true at the same
// time. We did not use an enum to make this invalid behavior unrepresentable
// because we can't use RefCell to borrow() the variants of the enum - only
// Cell provides mutability without pulling in panic machinery - so it would
// make the accessors a lot more awkward to write.
pub struct ImportAlloc {
    // When not-null, allocator should use this buffer/len pair at most once
    // to satisfy allocations.
    buffer: Cell<*mut u8>,
    len: Cell<usize>,
    // When not-empty, allocator should use this arena to satisfy allocations.
    arena: Cell<Option<&'static BumpArena>>,
}

impl ImportAlloc {
    fn new() -> Self {
        ImportAlloc {
            buffer: Cell::new(std::ptr::null_mut()),
            len: Cell::new(0),
            arena: Cell::new(None),
        }
    }

    /// Expect at most one import allocation during execution of the provided closure.
    /// Use the provided buffer to satisfy that import allocation. The user is responsible
    /// for making sure allocated imports are not used beyond the lifetime of the buffer.
    fn with_buffer<T>(&self, buffer: *mut u8, len: usize, f: impl FnOnce() -> T) -> T {
        if self.arena.get().is_some() {
            unreachable!("arena mode")
        }
        let prev = self.buffer.replace(buffer);
        if !prev.is_null() {
            unreachable!("overwrote another buffer")
        }
        self.len.set(len);
        let r = f();
        self.buffer.set(std::ptr::null_mut());
        r
    }

    /// Permit many import allocations during execution of the provided closure.
    /// Use the provided BumpArena to satisfry those allocations. The user is responsible
    /// for making sure allocated imports are not used beyond the lifetime of the arena.
    fn with_arena<T>(&self, arena: &BumpArena, f: impl FnOnce() -> T) -> T {
        if !self.buffer.get().is_null() {
            unreachable!("buffer mode")
        }
        let prev = self.arena.replace(Some(unsafe {
            // Safety: Need to erase the lifetime to store in the arena cell.
            std::mem::transmute::<&'_ BumpArena, &'static BumpArena>(arena)
        }));
        if prev.is_some() {
            unreachable!("overwrote another arena")
        }
        let r = f();
        self.arena.set(None);
        r
    }

    /// To be used by cabi_import_realloc only!
    fn alloc(&self, align: usize, size: usize) -> *mut u8 {
        if let Some(arena) = self.arena.get() {
            arena.alloc(align, size)
        } else {
            let buffer = self.buffer.get();
            if buffer.is_null() {
                unreachable!("buffer not provided, or already used")
            }
            let buffer = buffer as usize;
            let alloc = align_to(buffer, align);
            if alloc.checked_add(size).trapping_unwrap()
                > buffer.checked_add(self.len.get()).trapping_unwrap()
            {
                unreachable!("out of memory")
            }
            self.buffer.set(std::ptr::null_mut());
            alloc as *mut u8
        }
    }
}

/// This allocator is only used for the `run` entrypoint.
///
/// The implementation here is a bump allocator into `State::long_lived_arena` which
/// traps when it runs out of data. This means that the total size of
/// arguments/env/etc coming into a component is bounded by the current 64k
/// (ish) limit. That's just an implementation limit though which can be lifted
/// by dynamically calling the main module's allocator as necessary for more data.
#[no_mangle]
pub unsafe extern "C" fn cabi_export_realloc(
    old_ptr: *mut u8,
    old_size: usize,
    align: usize,
    new_size: usize,
) -> *mut u8 {
    if !old_ptr.is_null() || old_size != 0 {
        unreachable!();
    }
    let mut ret = null_mut::<u8>();
    State::with(|state| {
        ret = state.long_lived_arena.alloc(align, new_size);
        Ok(())
    });
    ret
}

/// Read command-line argument data.
/// The size of the array should match that returned by `args_sizes_get`
#[no_mangle]
pub unsafe extern "C" fn args_get(mut argv: *mut *mut u8, mut argv_buf: *mut u8) -> Errno {
    State::with(|state| {
        #[cfg(not(feature = "proxy"))]
        {
            for arg in state.get_args() {
                // Copy the argument into `argv_buf` which must be sized
                // appropriately by the caller.
                ptr::copy_nonoverlapping(arg.ptr, argv_buf, arg.len);
                *argv_buf.add(arg.len) = 0;

                // Copy the argument pointer into the `argv` buf
                *argv = argv_buf;

                // Update our pointers past what's written to prepare for the
                // next argument.
                argv = argv.add(1);
                argv_buf = argv_buf.add(arg.len + 1);
            }
        }
        Ok(())
    })
}

/// Return command-line argument data sizes.
#[no_mangle]
pub unsafe extern "C" fn args_sizes_get(argc: *mut Size, argv_buf_size: *mut Size) -> Errno {
    State::with(|state| {
        #[cfg(feature = "proxy")]
        {
            *argc = 0;
            *argv_buf_size = 0;
        }
        #[cfg(not(feature = "proxy"))]
        {
            let args = state.get_args();
            *argc = args.len();
            // Add one to each length for the terminating nul byte added by
            // the `args_get` function.
            *argv_buf_size = args.iter().map(|s| s.len + 1).sum();
        }
        Ok(())
    })
}

/// Read environment variable data.
/// The sizes of the buffers should match that returned by `environ_sizes_get`.
#[no_mangle]
pub unsafe extern "C" fn environ_get(environ: *mut *mut u8, environ_buf: *mut u8) -> Errno {
    State::with(|state| {
        #[cfg(not(feature = "proxy"))]
        {
            let mut offsets = environ;
            let mut buffer = environ_buf;
            for var in state.get_environment() {
                ptr::write(offsets, buffer);
                offsets = offsets.add(1);

                ptr::copy_nonoverlapping(var.key.ptr, buffer, var.key.len);
                buffer = buffer.add(var.key.len);

                ptr::write(buffer, b'=');
                buffer = buffer.add(1);

                ptr::copy_nonoverlapping(var.value.ptr, buffer, var.value.len);
                buffer = buffer.add(var.value.len);

                ptr::write(buffer, 0);
                buffer = buffer.add(1);
            }
        }

        Ok(())
    })
}

/// Return environment variable data sizes.
#[no_mangle]
pub unsafe extern "C" fn environ_sizes_get(
    environc: *mut Size,
    environ_buf_size: *mut Size,
) -> Errno {
    if !matches!(
        get_allocation_state(),
        AllocationState::StackAllocated | AllocationState::StateAllocated
    ) {
        *environc = 0;
        *environ_buf_size = 0;
        return ERRNO_SUCCESS;
    }

    State::with(|state| {
        #[cfg(feature = "proxy")]
        {
            *environc = 0;
            *environ_buf_size = 0;
        }
        #[cfg(not(feature = "proxy"))]
        {
            let vars = state.get_environment();
            *environc = vars.len();
            *environ_buf_size = {
                let mut sum = 0;
                for var in vars {
                    sum += var.key.len + var.value.len + 2;
                }
                sum
            };
        }

        Ok(())
    })
}

/// Return the resolution of a clock.
/// Implementations are required to provide a non-zero value for supported clocks. For unsupported clocks,
/// return `errno::inval`.
/// Note: This is similar to `clock_getres` in POSIX.
#[no_mangle]
pub extern "C" fn clock_res_get(id: Clockid, resolution: &mut Timestamp) -> Errno {
    match id {
        CLOCKID_MONOTONIC => {
            *resolution = monotonic_clock::resolution();
            ERRNO_SUCCESS
        }
        CLOCKID_REALTIME => {
            let res = wall_clock::resolution();
            *resolution = match Timestamp::from(res.seconds)
                .checked_mul(1_000_000_000)
                .and_then(|ns| ns.checked_add(res.nanoseconds.into()))
            {
                Some(ns) => ns,
                None => return ERRNO_OVERFLOW,
            };
            ERRNO_SUCCESS
        }
        _ => ERRNO_BADF,
    }
}

/// Return the time value of a clock.
/// Note: This is similar to `clock_gettime` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn clock_time_get(
    id: Clockid,
    _precision: Timestamp,
    time: &mut Timestamp,
) -> Errno {
    match id {
        CLOCKID_MONOTONIC => {
            *time = monotonic_clock::now();
            ERRNO_SUCCESS
        }
        CLOCKID_REALTIME => {
            let res = wall_clock::now();
            *time = match Timestamp::from(res.seconds)
                .checked_mul(1_000_000_000)
                .and_then(|ns| ns.checked_add(res.nanoseconds.into()))
            {
                Some(ns) => ns,
                None => return ERRNO_OVERFLOW,
            };
            ERRNO_SUCCESS
        }
        _ => ERRNO_BADF,
    }
}

/// Provide file advisory information on a file descriptor.
/// Note: This is similar to `posix_fadvise` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_advise(
    fd: Fd,
    offset: Filesize,
    len: Filesize,
    advice: Advice,
) -> Errno {
    cfg_filesystem_available! {
        let advice = match advice {
            ADVICE_NORMAL => filesystem::Advice::Normal,
            ADVICE_SEQUENTIAL => filesystem::Advice::Sequential,
            ADVICE_RANDOM => filesystem::Advice::Random,
            ADVICE_WILLNEED => filesystem::Advice::WillNeed,
            ADVICE_DONTNEED => filesystem::Advice::DontNeed,
            ADVICE_NOREUSE => filesystem::Advice::NoReuse,
            _ => return ERRNO_INVAL,
        };
        State::with(|state| {
            let ds = state.descriptors();
            let file = ds.get_seekable_file(fd)?;
            file.fd.advise(offset, len, advice)?;
            Ok(())
        })
    }
}

/// Force the allocation of space in a file.
/// Note: This is similar to `posix_fallocate` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_allocate(fd: Fd, _offset: Filesize, _len: Filesize) -> Errno {
    cfg_filesystem_available! {
        State::with(|state| {
            let ds = state.descriptors();
            // For not-files, fail with BADF
            ds.get_file(fd)?;
            // For all files, fail with NOTSUP, because this call does not exist in preview 2.
            Err(wasi::ERRNO_NOTSUP)
        })
    }
}

/// Close a file descriptor.
/// Note: This is similar to `close` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_close(fd: Fd) -> Errno {
    State::with(|state| {
        if let Descriptor::Bad = state.descriptors().get(fd)? {
            return Err(wasi::ERRNO_BADF);
        }

        // If there's a dirent cache entry for this file descriptor then drop
        // it since the descriptor is being closed and future calls to
        // `fd_readdir` should return an error.
        #[cfg(not(feature = "proxy"))]
        if fd == state.dirent_cache.for_fd.get() {
            drop(state.dirent_cache.stream.replace(None));
        }

        state.descriptors_mut().close(fd)?;
        Ok(())
    })
}

/// Synchronize the data of a file to disk.
/// Note: This is similar to `fdatasync` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_datasync(fd: Fd) -> Errno {
    cfg_filesystem_available! {
        State::with(|state| {
            let ds = state.descriptors();
            let file = ds.get_file(fd)?;
            file.fd.sync_data()?;
            Ok(())
        })
    }
}

/// Get the attributes of a file descriptor.
/// Note: This returns similar flags to `fsync(fd, F_GETFL)` in POSIX, as well as additional fields.
#[no_mangle]
pub unsafe extern "C" fn fd_fdstat_get(fd: Fd, stat: *mut Fdstat) -> Errno {
    cfg_filesystem_available! {
        State::with(|state| {
            let ds = state.descriptors();
            match ds.get(fd)? {
                Descriptor::Streams(Streams {
                    type_: StreamType::File(file),
                    ..
                }) => {
                    let flags = file.fd.get_flags()?;
                    let type_ = file.fd.get_type()?;
                    match type_ {
                        filesystem::DescriptorType::Directory => {
                            // Hard-coded set of rights expected by many userlands:
                            let fs_rights_base = wasi::RIGHTS_PATH_CREATE_DIRECTORY
                                | wasi::RIGHTS_PATH_CREATE_FILE
                                | wasi::RIGHTS_PATH_LINK_SOURCE
                                | wasi::RIGHTS_PATH_LINK_TARGET
                                | wasi::RIGHTS_PATH_OPEN
                                | wasi::RIGHTS_FD_READDIR
                                | wasi::RIGHTS_PATH_READLINK
                                | wasi::RIGHTS_PATH_RENAME_SOURCE
                                | wasi::RIGHTS_PATH_RENAME_TARGET
                                | wasi::RIGHTS_PATH_SYMLINK
                                | wasi::RIGHTS_PATH_REMOVE_DIRECTORY
                                | wasi::RIGHTS_PATH_UNLINK_FILE
                                | wasi::RIGHTS_PATH_FILESTAT_GET
                                | wasi::RIGHTS_PATH_FILESTAT_SET_TIMES
                                | wasi::RIGHTS_FD_FILESTAT_GET
                                | wasi::RIGHTS_FD_FILESTAT_SET_TIMES;

                            let fs_rights_inheriting = fs_rights_base
                                | wasi::RIGHTS_FD_DATASYNC
                                | wasi::RIGHTS_FD_READ
                                | wasi::RIGHTS_FD_SEEK
                                | wasi::RIGHTS_FD_FDSTAT_SET_FLAGS
                                | wasi::RIGHTS_FD_SYNC
                                | wasi::RIGHTS_FD_TELL
                                | wasi::RIGHTS_FD_WRITE
                                | wasi::RIGHTS_FD_ADVISE
                                | wasi::RIGHTS_FD_ALLOCATE
                                | wasi::RIGHTS_FD_FILESTAT_GET
                                | wasi::RIGHTS_FD_FILESTAT_SET_SIZE
                                | wasi::RIGHTS_FD_FILESTAT_SET_TIMES
                                | wasi::RIGHTS_POLL_FD_READWRITE;

                            stat.write(Fdstat {
                                fs_filetype: wasi::FILETYPE_DIRECTORY,
                                fs_flags: 0,
                                fs_rights_base,
                                fs_rights_inheriting,
                            });
                            Ok(())
                        }
                        _ => {
                            let fs_filetype = type_.into();

                            let mut fs_flags = 0;
                            let mut fs_rights_base = !0;
                            if !flags.contains(filesystem::DescriptorFlags::READ) {
                                fs_rights_base &= !RIGHTS_FD_READ;
                            }
                            if !flags.contains(filesystem::DescriptorFlags::WRITE) {
                                fs_rights_base &= !RIGHTS_FD_WRITE;
                            }
                            if flags.contains(filesystem::DescriptorFlags::DATA_INTEGRITY_SYNC) {
                                fs_flags |= FDFLAGS_DSYNC;
                            }
                            if flags.contains(filesystem::DescriptorFlags::REQUESTED_WRITE_SYNC) {
                                fs_flags |= FDFLAGS_RSYNC;
                            }
                            if flags.contains(filesystem::DescriptorFlags::FILE_INTEGRITY_SYNC) {
                                fs_flags |= FDFLAGS_SYNC;
                            }
                            if file.append {
                                fs_flags |= FDFLAGS_APPEND;
                            }
                            if matches!(file.blocking_mode, BlockingMode::NonBlocking) {
                                fs_flags |= FDFLAGS_NONBLOCK;
                            }
                            let fs_rights_inheriting = fs_rights_base;

                            stat.write(Fdstat {
                                fs_filetype,
                                fs_flags,
                                fs_rights_base,
                                fs_rights_inheriting,
                            });
                            Ok(())
                        }
                    }
                }
                Descriptor::Streams(Streams {
                    input,
                    output,
                    type_: StreamType::Stdio(stdio),
                }) => {
                    let fs_flags = 0;
                    let mut fs_rights_base = 0;
                    if input.get().is_some() {
                        fs_rights_base |= RIGHTS_FD_READ;
                    }
                    if output.get().is_some() {
                        fs_rights_base |= RIGHTS_FD_WRITE;
                    }
                    let fs_rights_inheriting = fs_rights_base;
                    stat.write(Fdstat {
                        fs_filetype: stdio.filetype(),
                        fs_flags,
                        fs_rights_base,
                        fs_rights_inheriting,
                    });
                    Ok(())
                }
                Descriptor::Closed(_) | Descriptor::Bad => Err(ERRNO_BADF),
            }
        })
    }
}

/// Adjust the flags associated with a file descriptor.
/// Note: This is similar to `fcntl(fd, F_SETFL, flags)` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_fdstat_set_flags(fd: Fd, flags: Fdflags) -> Errno {
    // Only support changing the NONBLOCK or APPEND flags.
    if flags & !(FDFLAGS_NONBLOCK | FDFLAGS_APPEND) != 0 {
        return wasi::ERRNO_INVAL;
    }

    cfg_filesystem_available! {
        State::with(|state| {
            let mut ds = state.descriptors_mut();
            let file = match ds.get_mut(fd)? {
                Descriptor::Streams(Streams {
                    type_: StreamType::File(file),
                    ..
                }) if !file.is_dir() => file,
                _ => Err(wasi::ERRNO_BADF)?,
            };
            file.append = flags & FDFLAGS_APPEND == FDFLAGS_APPEND;
            file.blocking_mode = if flags & FDFLAGS_NONBLOCK == FDFLAGS_NONBLOCK {
                BlockingMode::NonBlocking
            } else {
                BlockingMode::Blocking
            };
            Ok(())
        })
    }
}

/// Does not do anything if `fd` corresponds to a valid descriptor and returns [`wasi::ERRNO_BADF`] otherwise.
#[no_mangle]
pub unsafe extern "C" fn fd_fdstat_set_rights(
    fd: Fd,
    _fs_rights_base: Rights,
    _fs_rights_inheriting: Rights,
) -> Errno {
    State::with(|state| {
        let ds = state.descriptors();
        match ds.get(fd)? {
            Descriptor::Streams(..) => Ok(()),
            Descriptor::Closed(..) | Descriptor::Bad => Err(wasi::ERRNO_BADF),
        }
    })
}

/// Return the attributes of an open file.
#[no_mangle]
pub unsafe extern "C" fn fd_filestat_get(fd: Fd, buf: *mut Filestat) -> Errno {
    cfg_filesystem_available! {
        State::with(|state| {
            let ds = state.descriptors();
            match ds.get(fd)? {
                Descriptor::Streams(Streams {
                    type_: StreamType::File(file),
                    ..
                }) => {
                    let stat = file.fd.stat()?;
                    let metadata_hash = file.fd.metadata_hash()?;
                    let filetype = stat.type_.into();
                    *buf = Filestat {
                        dev: 1,
                        ino: metadata_hash.lower,
                        filetype,
                        nlink: stat.link_count,
                        size: stat.size,
                        atim: datetime_to_timestamp(stat.data_access_timestamp),
                        mtim: datetime_to_timestamp(stat.data_modification_timestamp),
                        ctim: datetime_to_timestamp(stat.status_change_timestamp),
                    };
                    Ok(())
                }
                // Stdio is all zero fields, except for filetype character device
                Descriptor::Streams(Streams {
                    type_: StreamType::Stdio(stdio),
                    ..
                }) => {
                    *buf = Filestat {
                        dev: 0,
                        ino: 0,
                        filetype: stdio.filetype(),
                        nlink: 0,
                        size: 0,
                        atim: 0,
                        mtim: 0,
                        ctim: 0,
                    };
                    Ok(())
                }
                _ => Err(wasi::ERRNO_BADF),
            }
        })
    }
}

/// Adjust the size of an open file. If this increases the file's size, the extra bytes are filled with zeros.
/// Note: This is similar to `ftruncate` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_filestat_set_size(fd: Fd, size: Filesize) -> Errno {
    cfg_filesystem_available! {
        State::with(|state| {
            let ds = state.descriptors();
            let file = ds.get_file(fd)?;
            file.fd.set_size(size)?;
            Ok(())
        })
    }
}

#[cfg(not(feature = "proxy"))]
fn systimespec(set: bool, ts: Timestamp, now: bool) -> Result<filesystem::NewTimestamp, Errno> {
    if set && now {
        Err(wasi::ERRNO_INVAL)
    } else if set {
        Ok(filesystem::NewTimestamp::Timestamp(filesystem::Datetime {
            seconds: ts / 1_000_000_000,
            nanoseconds: (ts % 1_000_000_000) as _,
        }))
    } else if now {
        Ok(filesystem::NewTimestamp::Now)
    } else {
        Ok(filesystem::NewTimestamp::NoChange)
    }
}

/// Adjust the timestamps of an open file or directory.
/// Note: This is similar to `futimens` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_filestat_set_times(
    fd: Fd,
    atim: Timestamp,
    mtim: Timestamp,
    fst_flags: Fstflags,
) -> Errno {
    cfg_filesystem_available! {
        State::with(|state| {
            let atim = systimespec(
                fst_flags & FSTFLAGS_ATIM == FSTFLAGS_ATIM,
                atim,
                fst_flags & FSTFLAGS_ATIM_NOW == FSTFLAGS_ATIM_NOW,
            )?;
            let mtim = systimespec(
                fst_flags & FSTFLAGS_MTIM == FSTFLAGS_MTIM,
                mtim,
                fst_flags & FSTFLAGS_MTIM_NOW == FSTFLAGS_MTIM_NOW,
            )?;
            let ds = state.descriptors();
            let file = ds.get_file(fd)?;
            file.fd.set_times(atim, mtim)?;
            Ok(())
        })
    }
}

/// Read from a file descriptor, without using and updating the file descriptor's offset.
/// Note: This is similar to `preadv` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_pread(
    fd: Fd,
    mut iovs_ptr: *const Iovec,
    mut iovs_len: usize,
    offset: Filesize,
    nread: *mut Size,
) -> Errno {
    cfg_filesystem_available! {
        // Advance to the first non-empty buffer.
        while iovs_len != 0 && (*iovs_ptr).buf_len == 0 {
            iovs_ptr = iovs_ptr.add(1);
            iovs_len -= 1;
        }
        if iovs_len == 0 {
            *nread = 0;
            return ERRNO_SUCCESS;
        }

        State::with(|state| {
            let ptr = (*iovs_ptr).buf;
            let len = (*iovs_ptr).buf_len;

            let ds = state.descriptors();
            let file = ds.get_file(fd)?;
            let (data, end) = state
                .import_alloc
                .with_buffer(ptr, len, || file.fd.read(len as u64, offset))?;
            assert_eq!(data.as_ptr(), ptr);
            assert!(data.len() <= len);

            let len = data.len();
            forget(data);
            if !end && len == 0 {
                Err(ERRNO_INTR)
            } else {
                *nread = len;
                Ok(())
            }
        })
    }
}

/// Return a description of the given preopened file descriptor.
#[no_mangle]
pub unsafe extern "C" fn fd_prestat_get(fd: Fd, buf: *mut Prestat) -> Errno {
    if !matches!(
        get_allocation_state(),
        AllocationState::StackAllocated | AllocationState::StateAllocated
    ) {
        return ERRNO_BADF;
    }

    cfg_filesystem_available! {
        State::with(|state| {
            let ds = state.descriptors();
            if let Some(preopen) = ds.get_preopen(fd) {
                buf.write(Prestat {
                    tag: 0,
                    u: PrestatU {
                        dir: PrestatDir {
                            pr_name_len: preopen.path.len,
                        },
                    },
                });

                Ok(())
            } else {
                Err(ERRNO_BADF)
            }
        })
    }
}

/// Return a description of the given preopened file descriptor.
#[no_mangle]
pub unsafe extern "C" fn fd_prestat_dir_name(fd: Fd, path: *mut u8, path_max_len: Size) -> Errno {
    cfg_filesystem_available! {
        State::with(|state| {
            let ds = state.descriptors();
            if let Some(preopen) = ds.get_preopen(fd) {
                if preopen.path.len > path_max_len {
                    Err(ERRNO_NAMETOOLONG)
                } else {
                    ptr::copy_nonoverlapping(preopen.path.ptr, path, preopen.path.len);
                    Ok(())
                }
            } else {
                Err(ERRNO_NOTDIR)
            }
        })
    }
}

/// Write to a file descriptor, without using and updating the file descriptor's offset.
/// Note: This is similar to `pwritev` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_pwrite(
    fd: Fd,
    mut iovs_ptr: *const Ciovec,
    mut iovs_len: usize,
    offset: Filesize,
    nwritten: *mut Size,
) -> Errno {
    cfg_filesystem_available! {
        // Advance to the first non-empty buffer.
        while iovs_len != 0 && (*iovs_ptr).buf_len == 0 {
            iovs_ptr = iovs_ptr.add(1);
            iovs_len -= 1;
        }
        if iovs_len == 0 {
            *nwritten = 0;
            return ERRNO_SUCCESS;
        }

        let ptr = (*iovs_ptr).buf;
        let len = (*iovs_ptr).buf_len;

        State::with(|state| {
            let ds = state.descriptors();
            let file = ds.get_seekable_file(fd)?;
            let bytes = file.fd.write(slice::from_raw_parts(ptr, len), offset)?;
            *nwritten = bytes as usize;
            Ok(())
        })
    }
}

/// Read from a file descriptor.
/// Note: This is similar to `readv` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_read(
    fd: Fd,
    mut iovs_ptr: *const Iovec,
    mut iovs_len: usize,
    nread: *mut Size,
) -> Errno {
    // Advance to the first non-empty buffer.
    while iovs_len != 0 && (*iovs_ptr).buf_len == 0 {
        iovs_ptr = iovs_ptr.add(1);
        iovs_len -= 1;
    }
    if iovs_len == 0 {
        *nread = 0;
        return ERRNO_SUCCESS;
    }

    let ptr = (*iovs_ptr).buf;
    let len = (*iovs_ptr).buf_len;

    State::with(|state| {
        let ds = state.descriptors();
        match ds.get(fd)? {
            Descriptor::Streams(streams) => {
                #[cfg(not(feature = "proxy"))]
                let blocking_mode = if let StreamType::File(file) = &streams.type_ {
                    file.blocking_mode
                } else {
                    BlockingMode::Blocking
                };
                #[cfg(feature = "proxy")]
                let blocking_mode = BlockingMode::Blocking;

                let read_len = u64::try_from(len).trapping_unwrap();
                let wasi_stream = streams.get_read_stream()?;
                let data = match state
                    .import_alloc
                    .with_buffer(ptr, len, || blocking_mode.read(wasi_stream, read_len))
                {
                    Ok(data) => data,
                    Err(streams::StreamError::Closed) => {
                        *nread = 0;
                        return Ok(());
                    }
                    Err(streams::StreamError::LastOperationFailed(e)) => {
                        Err(stream_error_to_errno(e))?
                    }
                };

                assert_eq!(data.as_ptr(), ptr);
                assert!(data.len() <= len);

                // If this is a file, keep the current-position pointer up to date.
                #[cfg(not(feature = "proxy"))]
                if let StreamType::File(file) = &streams.type_ {
                    file.position
                        .set(file.position.get() + data.len() as filesystem::Filesize);
                    if len == 0 {
                        return Err(ERRNO_INTR);
                    }
                }

                let len = data.len();
                *nread = len;
                forget(data);
                Ok(())
            }
            Descriptor::Closed(_) | Descriptor::Bad => Err(ERRNO_BADF),
        }
    })
}

fn stream_error_to_errno(err: streams::Error) -> Errno {
    #[cfg(feature = "proxy")]
    return ERRNO_IO;
    #[cfg(not(feature = "proxy"))]
    match filesystem::filesystem_error_code(&err) {
        Some(code) => code.into(),
        None => ERRNO_IO,
    }
}

/// Read directory entries from a directory.
/// When successful, the contents of the output buffer consist of a sequence of
/// directory entries. Each directory entry consists of a `dirent` object,
/// followed by `dirent::d_namlen` bytes holding the name of the directory
/// entry.
/// This function fills the output buffer as much as possible, potentially
/// truncating the last directory entry. This allows the caller to grow its
/// read buffer size in case it's too small to fit a single large directory
/// entry, or skip the oversized directory entry.
#[no_mangle]
#[cfg(feature = "proxy")]
pub unsafe extern "C" fn fd_readdir(
    fd: Fd,
    buf: *mut u8,
    buf_len: Size,
    cookie: Dircookie,
    bufused: *mut Size,
) -> Errno {
    wasi::ERRNO_NOTSUP
}

#[no_mangle]
#[cfg(not(feature = "proxy"))]
pub unsafe extern "C" fn fd_readdir(
    fd: Fd,
    buf: *mut u8,
    buf_len: Size,
    cookie: Dircookie,
    bufused: *mut Size,
) -> Errno {
    let mut buf = slice::from_raw_parts_mut(buf, buf_len);
    return State::with(|state| {
        // First determine if there's an entry in the dirent cache to use. This
        // is done to optimize the use case where a large directory is being
        // used with a fixed-sized buffer to avoid re-invoking the `readdir`
        // function and continuing to use the same iterator.
        //
        // This is a bit tricky since the requested state in this function call
        // must match the prior state of the dirent stream, if any, so that's
        // all validated here as well.
        //
        // Note that for the duration of this function the `cookie` specifier is
        // the `n`th iteration of the `readdir` stream return value.
        let prev_stream = state.dirent_cache.stream.replace(None);
        let stream =
            if state.dirent_cache.for_fd.get() == fd && state.dirent_cache.cookie.get() == cookie {
                prev_stream
            } else {
                None
            };

        // Compute the inode of `.` so that the iterator can produce an entry
        // for it.
        let ds = state.descriptors();
        let dir = ds.get_dir(fd)?;

        let mut iter;
        match stream {
            // All our checks passed and a dirent cache was available with a
            // prior stream. Construct an iterator which will yield its first
            // entry from cache and is additionally resuming at the `cookie`
            // specified.
            Some(stream) => {
                iter = DirectoryEntryIterator {
                    stream,
                    state,
                    cookie,
                    use_cache: true,
                    dir_descriptor: &dir.fd,
                }
            }

            // Either a dirent stream wasn't previously available, a different
            // cookie was requested, or a brand new directory is now being read.
            // In these situations fall back to resuming reading the directory
            // from scratch, and the `cookie` value indicates how many items
            // need skipping.
            None => {
                iter = DirectoryEntryIterator {
                    state,
                    cookie: wasi::DIRCOOKIE_START,
                    use_cache: false,
                    stream: DirectoryEntryStream(dir.fd.read_directory()?),
                    dir_descriptor: &dir.fd,
                };

                // Skip to the entry that is requested by the `cookie`
                // parameter.
                for _ in wasi::DIRCOOKIE_START..cookie {
                    match iter.next() {
                        Some(Ok(_)) => {}
                        Some(Err(e)) => return Err(e),
                        None => return Ok(()),
                    }
                }
            }
        };

        while buf.len() > 0 {
            let (dirent, name) = match iter.next() {
                Some(Ok(pair)) => pair,
                Some(Err(e)) => return Err(e),
                None => break,
            };

            // Copy a `dirent` describing this entry into the destination `buf`,
            // truncating it if it doesn't fit entirely.
            let bytes = slice::from_raw_parts(
                (&dirent as *const wasi::Dirent).cast::<u8>(),
                size_of::<Dirent>(),
            );
            let dirent_bytes_to_copy = buf.len().min(bytes.len());
            buf[..dirent_bytes_to_copy].copy_from_slice(&bytes[..dirent_bytes_to_copy]);
            buf = &mut buf[dirent_bytes_to_copy..];

            // Copy the name bytes into the output `buf`, truncating it if it
            // doesn't fit.
            //
            // Note that this might be a 0-byte copy if the `dirent` was
            // truncated or fit entirely into the destination.
            let name_bytes_to_copy = buf.len().min(name.len());
            ptr::copy_nonoverlapping(name.as_ptr().cast(), buf.as_mut_ptr(), name_bytes_to_copy);

            buf = &mut buf[name_bytes_to_copy..];

            // If the buffer is empty then that means the value may be
            // truncated, so save the state of the iterator in our dirent cache
            // and return.
            //
            // Note that `cookie - 1` is stored here since `iter.cookie` stores
            // the address of the next item, and we're rewinding one item since
            // the current item is truncated and will want to resume from that
            // in the future.
            //
            // Additionally note that this caching step is skipped if the name
            // to store doesn't actually fit in the dirent cache's path storage.
            // In that case there's not much we can do and let the next call to
            // `fd_readdir` start from scratch.
            if buf.len() == 0 && name.len() <= DIRENT_CACHE {
                let DirectoryEntryIterator { stream, cookie, .. } = iter;
                state.dirent_cache.stream.set(Some(stream));
                state.dirent_cache.for_fd.set(fd);
                state.dirent_cache.cookie.set(cookie - 1);
                state.dirent_cache.cached_dirent.set(dirent);
                ptr::copy(
                    name.as_ptr().cast::<u8>(),
                    (*state.dirent_cache.path_data.get()).as_mut_ptr() as *mut u8,
                    name.len(),
                );
                break;
            }
        }

        *bufused = buf_len - buf.len();
        Ok(())
    });

    struct DirectoryEntryIterator<'a> {
        state: &'a State,
        use_cache: bool,
        cookie: Dircookie,
        stream: DirectoryEntryStream,
        dir_descriptor: &'a filesystem::Descriptor,
    }

    impl<'a> Iterator for DirectoryEntryIterator<'a> {
        // Note the usage of `UnsafeCell<u8>` here to indicate that the data can
        // alias the storage within `state`.
        type Item = Result<(wasi::Dirent, &'a [UnsafeCell<u8>]), Errno>;

        fn next(&mut self) -> Option<Self::Item> {
            let current_cookie = self.cookie;

            self.cookie += 1;

            // Preview1 programs expect to see `.` and `..` in the traversal, but
            // Preview2 excludes them, so re-add them.
            match current_cookie {
                0 => {
                    let metadata_hash = match self.dir_descriptor.metadata_hash() {
                        Ok(h) => h,
                        Err(e) => return Some(Err(e.into())),
                    };
                    let dirent = wasi::Dirent {
                        d_next: self.cookie,
                        d_ino: metadata_hash.lower,
                        d_type: wasi::FILETYPE_DIRECTORY,
                        d_namlen: 1,
                    };
                    return Some(Ok((dirent, &self.state.dotdot[..1])));
                }
                1 => {
                    let dirent = wasi::Dirent {
                        d_next: self.cookie,
                        d_ino: 0,
                        d_type: wasi::FILETYPE_DIRECTORY,
                        d_namlen: 2,
                    };
                    return Some(Ok((dirent, &self.state.dotdot[..])));
                }
                _ => {}
            }

            if self.use_cache {
                self.use_cache = false;
                return Some(unsafe {
                    let dirent = self.state.dirent_cache.cached_dirent.as_ptr().read();
                    let ptr = (*(*self.state.dirent_cache.path_data.get()).as_ptr())
                        .as_ptr()
                        .cast();
                    let buffer = slice::from_raw_parts(ptr, dirent.d_namlen as usize);
                    Ok((dirent, buffer))
                });
            }
            let entry = self.state.import_alloc.with_buffer(
                self.state.path_buf.get().cast(),
                PATH_MAX,
                || self.stream.0.read_directory_entry(),
            );
            let entry = match entry {
                Ok(Some(entry)) => entry,
                Ok(None) => return None,
                Err(e) => return Some(Err(e.into())),
            };

            let filesystem::DirectoryEntry { type_, name } = entry;
            let d_ino = self
                .dir_descriptor
                .metadata_hash_at(filesystem::PathFlags::empty(), &name)
                .map(|h| h.lower)
                .unwrap_or(0);
            let name = ManuallyDrop::new(name);
            let dirent = wasi::Dirent {
                d_next: self.cookie,
                d_ino,
                d_namlen: u32::try_from(name.len()).trapping_unwrap(),
                d_type: type_.into(),
            };
            // Extend the lifetime of `name` to the `self.state` lifetime for
            // this iterator since the data for the name lives within state.
            let name = unsafe {
                assert_eq!(name.as_ptr(), self.state.path_buf.get().cast());
                slice::from_raw_parts(name.as_ptr().cast(), name.len())
            };
            Some(Ok((dirent, name)))
        }
    }
}

/// Atomically replace a file descriptor by renumbering another file descriptor.
/// Due to the strong focus on thread safety, this environment does not provide
/// a mechanism to duplicate or renumber a file descriptor to an arbitrary
/// number, like `dup2()`. This would be prone to race conditions, as an actual
/// file descriptor with the same number could be allocated by a different
/// thread at the same time.
/// This function provides a way to atomically renumber file descriptors, which
/// would disappear if `dup2()` were to be removed entirely.
#[no_mangle]
pub unsafe extern "C" fn fd_renumber(fd: Fd, to: Fd) -> Errno {
    State::with(|state| state.descriptors_mut().renumber(fd, to))
}

/// Move the offset of a file descriptor.
/// Note: This is similar to `lseek` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_seek(
    fd: Fd,
    offset: Filedelta,
    whence: Whence,
    newoffset: *mut Filesize,
) -> Errno {
    cfg_filesystem_available! {
        State::with(|state| {
            let mut ds = state.descriptors_mut();
            let stream = ds.get_seekable_stream_mut(fd)?;

            // Seeking only works on files.
            if let StreamType::File(file) = &mut stream.type_ {
                if let filesystem::DescriptorType::Directory = file.descriptor_type {
                    // This isn't really the "right" errno, but it is consistient with wasmtime's
                    // preview 1 tests.
                    return Err(ERRNO_BADF);
                }
                let from = match whence {
                    WHENCE_SET if offset >= 0 => offset,
                    WHENCE_CUR => match (file.position.get() as i64).checked_add(offset) {
                        Some(pos) if pos >= 0 => pos,
                        _ => return Err(ERRNO_INVAL),
                    },
                    WHENCE_END => match (file.fd.stat()?.size as i64).checked_add(offset) {
                        Some(pos) if pos >= 0 => pos,
                        _ => return Err(ERRNO_INVAL),
                    },
                    _ => return Err(ERRNO_INVAL),
                };
                drop(stream.input.take());
                drop(stream.output.take());
                file.position.set(from as filesystem::Filesize);
                *newoffset = from as filesystem::Filesize;
                Ok(())
            } else {
                Err(ERRNO_SPIPE)
            }
        })
    }
}

/// Synchronize the data and metadata of a file to disk.
/// Note: This is similar to `fsync` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_sync(fd: Fd) -> Errno {
    cfg_filesystem_available! {
        State::with(|state| {
            let ds = state.descriptors();
            let file = ds.get_file(fd)?;
            file.fd.sync()?;
            Ok(())
        })
    }
}

/// Return the current offset of a file descriptor.
/// Note: This is similar to `lseek(fd, 0, SEEK_CUR)` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_tell(fd: Fd, offset: *mut Filesize) -> Errno {
    cfg_filesystem_available! {
        State::with(|state| {
            let ds = state.descriptors();
            let file = ds.get_seekable_file(fd)?;
            *offset = file.position.get();
            Ok(())
        })
    }
}

/// Write to a file descriptor.
/// Note: This is similar to `writev` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn fd_write(
    fd: Fd,
    mut iovs_ptr: *const Ciovec,
    mut iovs_len: usize,
    nwritten: *mut Size,
) -> Errno {
    if !matches!(
        get_allocation_state(),
        AllocationState::StackAllocated | AllocationState::StateAllocated
    ) {
        *nwritten = 0;
        return ERRNO_IO;
    }

    // Advance to the first non-empty buffer.
    while iovs_len != 0 && (*iovs_ptr).buf_len == 0 {
        iovs_ptr = iovs_ptr.add(1);
        iovs_len -= 1;
    }
    if iovs_len == 0 {
        *nwritten = 0;
        return ERRNO_SUCCESS;
    }

    let ptr = (*iovs_ptr).buf;
    let len = (*iovs_ptr).buf_len;
    let bytes = slice::from_raw_parts(ptr, len);

    State::with(|state| {
        let ds = state.descriptors();
        match ds.get(fd)? {
            Descriptor::Streams(streams) => {
                let wasi_stream = streams.get_write_stream()?;

                #[cfg(not(feature = "proxy"))]
                let nbytes = if let StreamType::File(file) = &streams.type_ {
                    file.blocking_mode.write(wasi_stream, bytes)?
                } else {
                    // Use blocking writes on non-file streams (stdout, stderr, as sockets
                    // aren't currently used).
                    BlockingMode::Blocking.write(wasi_stream, bytes)?
                };
                #[cfg(feature = "proxy")]
                let nbytes = BlockingMode::Blocking.write(wasi_stream, bytes)?;

                // If this is a file, keep the current-position pointer up
                // to date. Note that for files that perform appending
                // writes this function will always update the current
                // position to the end of the file.
                //
                // NB: this isn't "atomic" as it doesn't necessarily account
                // for concurrent writes, but there's not much that can be
                // done about that.
                #[cfg(not(feature = "proxy"))]
                if let StreamType::File(file) = &streams.type_ {
                    if file.append {
                        file.position.set(file.fd.stat()?.size);
                    } else {
                        file.position.set(file.position.get() + nbytes as u64);
                    }
                }

                *nwritten = nbytes;
                Ok(())
            }
            Descriptor::Closed(_) | Descriptor::Bad => Err(ERRNO_BADF),
        }
    })
}

/// Create a directory.
/// Note: This is similar to `mkdirat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_create_directory(
    fd: Fd,
    path_ptr: *const u8,
    path_len: usize,
) -> Errno {
    cfg_filesystem_available! {
        let path = slice::from_raw_parts(path_ptr, path_len);

        State::with(|state| {
            let ds = state.descriptors();
            let file = ds.get_dir(fd)?;
            file.fd.create_directory_at(path)?;
            Ok(())
        })
    }
}

/// Return the attributes of a file or directory.
/// Note: This is similar to `stat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_filestat_get(
    fd: Fd,
    flags: Lookupflags,
    path_ptr: *const u8,
    path_len: usize,
    buf: *mut Filestat,
) -> Errno {
    cfg_filesystem_available! {
        let path = slice::from_raw_parts(path_ptr, path_len);
        let at_flags = at_flags_from_lookupflags(flags);

        State::with(|state| {
            let ds = state.descriptors();
            let file = ds.get_dir(fd)?;
            let stat = file.fd.stat_at(at_flags, path)?;
            let metadata_hash = file.fd.metadata_hash_at(at_flags, path)?;
            let filetype = stat.type_.into();
            *buf = Filestat {
                dev: 1,
                ino: metadata_hash.lower,
                filetype,
                nlink: stat.link_count,
                size: stat.size,
                atim: datetime_to_timestamp(stat.data_access_timestamp),
                mtim: datetime_to_timestamp(stat.data_modification_timestamp),
                ctim: datetime_to_timestamp(stat.status_change_timestamp),
            };
            Ok(())
        })
    }
}

/// Adjust the timestamps of a file or directory.
/// Note: This is similar to `utimensat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_filestat_set_times(
    fd: Fd,
    flags: Lookupflags,
    path_ptr: *const u8,
    path_len: usize,
    atim: Timestamp,
    mtim: Timestamp,
    fst_flags: Fstflags,
) -> Errno {
    cfg_filesystem_available! {
        let path = slice::from_raw_parts(path_ptr, path_len);
        let at_flags = at_flags_from_lookupflags(flags);

        State::with(|state| {
            let atim = systimespec(
                fst_flags & FSTFLAGS_ATIM == FSTFLAGS_ATIM,
                atim,
                fst_flags & FSTFLAGS_ATIM_NOW == FSTFLAGS_ATIM_NOW,
            )?;
            let mtim = systimespec(
                fst_flags & FSTFLAGS_MTIM == FSTFLAGS_MTIM,
                mtim,
                fst_flags & FSTFLAGS_MTIM_NOW == FSTFLAGS_MTIM_NOW,
            )?;

            let ds = state.descriptors();
            let file = ds.get_dir(fd)?;
            file.fd.set_times_at(at_flags, path, atim, mtim)?;
            Ok(())
        })
    }
}

/// Create a hard link.
/// Note: This is similar to `linkat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_link(
    old_fd: Fd,
    old_flags: Lookupflags,
    old_path_ptr: *const u8,
    old_path_len: usize,
    new_fd: Fd,
    new_path_ptr: *const u8,
    new_path_len: usize,
) -> Errno {
    cfg_filesystem_available! {
        let old_path = slice::from_raw_parts(old_path_ptr, old_path_len);
        let new_path = slice::from_raw_parts(new_path_ptr, new_path_len);
        let at_flags = at_flags_from_lookupflags(old_flags);

        State::with(|state| {
            let ds = state.descriptors();
            let old = &ds.get_dir(old_fd)?.fd;
            let new = &ds.get_dir(new_fd)?.fd;
            old.link_at(at_flags, old_path, new, new_path)?;
            Ok(())
        })
    }
}

/// Open a file or directory.
/// The returned file descriptor is not guaranteed to be the lowest-numbered
/// file descriptor not currently open; it is randomized to prevent
/// applications from depending on making assumptions about indexes, since this
/// is error-prone in multi-threaded contexts. The returned file descriptor is
/// guaranteed to be less than 2**31.
/// Note: This is similar to `openat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_open(
    fd: Fd,
    dirflags: Lookupflags,
    path_ptr: *const u8,
    path_len: usize,
    oflags: Oflags,
    fs_rights_base: Rights,
    fs_rights_inheriting: Rights,
    fdflags: Fdflags,
    opened_fd: *mut Fd,
) -> Errno {
    cfg_filesystem_available! {
        let _ = fs_rights_inheriting;

        let path = slice::from_raw_parts(path_ptr, path_len);
        let at_flags = at_flags_from_lookupflags(dirflags);
        let o_flags = o_flags_from_oflags(oflags);
        let flags = descriptor_flags_from_flags(fs_rights_base, fdflags);
        let append = fdflags & wasi::FDFLAGS_APPEND == wasi::FDFLAGS_APPEND;

        #[cfg(feature = "proxy")]
        return wasi::ERRNO_NOTSUP;

        #[cfg(not(feature = "proxy"))]
        State::with(|state| {
            let result = state
                .descriptors()
                .get_dir(fd)?
                .fd
                .open_at(at_flags, path, o_flags, flags)?;
            let descriptor_type = result.get_type()?;
            let desc = Descriptor::Streams(Streams {
                input: OnceCell::new(),
                output: OnceCell::new(),
                type_: StreamType::File(File {
                    fd: result,
                    descriptor_type,
                    position: Cell::new(0),
                    append,
                    blocking_mode: if fdflags & wasi::FDFLAGS_NONBLOCK == 0 {
                        BlockingMode::Blocking
                    } else {
                        BlockingMode::NonBlocking
                    },
                }),
            });

            let fd = state.descriptors_mut().open(desc)?;
            *opened_fd = fd;
            Ok(())
        })
    }
}

/// Read the contents of a symbolic link.
/// Note: This is similar to `readlinkat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_readlink(
    fd: Fd,
    path_ptr: *const u8,
    path_len: usize,
    buf: *mut u8,
    buf_len: Size,
    bufused: *mut Size,
) -> Errno {
    cfg_filesystem_available! {
        let path = slice::from_raw_parts(path_ptr, path_len);

        State::with(|state| {
            // If the user gave us a buffer shorter than `PATH_MAX`, it may not be
            // long enough to accept the actual path. `cabi_realloc` can't fail,
            // so instead we handle this case specially.
            let use_state_buf = buf_len < PATH_MAX;

            let ds = state.descriptors();
            let file = ds.get_dir(fd)?;
            let path = if use_state_buf {
                state
                    .import_alloc
                    .with_buffer(state.path_buf.get().cast(), PATH_MAX, || {
                        file.fd.readlink_at(path)
                    })?
            } else {
                state
                    .import_alloc
                    .with_buffer(buf, buf_len, || file.fd.readlink_at(path))?
            };

            if use_state_buf {
                // Preview1 follows POSIX in truncating the returned path if it
                // doesn't fit.
                let len = min(path.len(), buf_len);
                ptr::copy_nonoverlapping(path.as_ptr().cast(), buf, len);
                *bufused = len;
            } else {
                *bufused = path.len();
            }

            // The returned string's memory was allocated in `buf`, so don't separately
            // free it.
            forget(path);

            Ok(())
        })
    }
}

/// Remove a directory.
/// Return `errno::notempty` if the directory is not empty.
/// Note: This is similar to `unlinkat(fd, path, AT_REMOVEDIR)` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_remove_directory(
    fd: Fd,
    path_ptr: *const u8,
    path_len: usize,
) -> Errno {
    cfg_filesystem_available! {
        let path = slice::from_raw_parts(path_ptr, path_len);

        State::with(|state| {
            let ds = state.descriptors();
            let file = ds.get_dir(fd)?;
            file.fd.remove_directory_at(path)?;
            Ok(())
        })
    }
}

/// Rename a file or directory.
/// Note: This is similar to `renameat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_rename(
    old_fd: Fd,
    old_path_ptr: *const u8,
    old_path_len: usize,
    new_fd: Fd,
    new_path_ptr: *const u8,
    new_path_len: usize,
) -> Errno {
    cfg_filesystem_available! {
        let old_path = slice::from_raw_parts(old_path_ptr, old_path_len);
        let new_path = slice::from_raw_parts(new_path_ptr, new_path_len);

        State::with(|state| {
            let ds = state.descriptors();
            let old = &ds.get_dir(old_fd)?.fd;
            let new = &ds.get_dir(new_fd)?.fd;
            old.rename_at(old_path, new, new_path)?;
            Ok(())
        })
    }
}

/// Create a symbolic link.
/// Note: This is similar to `symlinkat` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_symlink(
    old_path_ptr: *const u8,
    old_path_len: usize,
    fd: Fd,
    new_path_ptr: *const u8,
    new_path_len: usize,
) -> Errno {
    cfg_filesystem_available! {
        let old_path = slice::from_raw_parts(old_path_ptr, old_path_len);
        let new_path = slice::from_raw_parts(new_path_ptr, new_path_len);

        State::with(|state| {
            let ds = state.descriptors();
            let file = ds.get_dir(fd)?;
            file.fd.symlink_at(old_path, new_path)?;
            Ok(())
        })
    }
}

/// Unlink a file.
/// Return `errno::isdir` if the path refers to a directory.
/// Note: This is similar to `unlinkat(fd, path, 0)` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn path_unlink_file(fd: Fd, path_ptr: *const u8, path_len: usize) -> Errno {
    cfg_filesystem_available! {
        let path = slice::from_raw_parts(path_ptr, path_len);

        State::with(|state| {
            let ds = state.descriptors();
            let file = ds.get_dir(fd)?;
            file.fd.unlink_file_at(path)?;
            Ok(())
        })
    }
}

struct Pollables {
    pointer: *mut Pollable,
    index: usize,
    length: usize,
}

impl Pollables {
    unsafe fn push(&mut self, pollable: Pollable) {
        assert!(self.index < self.length);
        // Use `ptr::write` instead of `*... = pollable` because `ptr::write`
        // doesn't call drop on the old memory.
        self.pointer.add(self.index).write(pollable);
        self.index += 1;
    }
}

// We create new pollable handles for each `poll_oneoff` call, so drop them all
// after the call.
impl Drop for Pollables {
    fn drop(&mut self) {
        while self.index != 0 {
            self.index -= 1;
            unsafe {
                core::ptr::drop_in_place(self.pointer.add(self.index));
            }
        }
    }
}

/// Concurrently poll for the occurrence of a set of events.
#[no_mangle]
pub unsafe extern "C" fn poll_oneoff(
    r#in: *const Subscription,
    out: *mut Event,
    nsubscriptions: Size,
    nevents: *mut Size,
) -> Errno {
    *nevents = 0;

    let subscriptions = slice::from_raw_parts(r#in, nsubscriptions);

    // We're going to split the `nevents` buffer into two non-overlapping
    // buffers: one to store the pollable handles, and the other to store
    // the bool results.
    //
    // First, we assert that this is possible:
    assert!(align_of::<Event>() >= align_of::<Pollable>());
    assert!(align_of::<Pollable>() >= align_of::<u32>());
    assert!(
        nsubscriptions
            .checked_mul(size_of::<Event>())
            .trapping_unwrap()
            >= nsubscriptions
                .checked_mul(size_of::<Pollable>())
                .trapping_unwrap()
                .checked_add(
                    nsubscriptions
                        .checked_mul(size_of::<u32>())
                        .trapping_unwrap()
                )
                .trapping_unwrap()
    );
    // Store the pollable handles at the beginning, and the bool results at the
    // end, so that we don't clobber the bool results when writting the events.
    let pollables = out as *mut c_void as *mut Pollable;
    let results = out.add(nsubscriptions).cast::<u32>().sub(nsubscriptions);

    // Indefinite sleeping is not supported in preview1.
    if nsubscriptions == 0 {
        return ERRNO_INVAL;
    }

    State::with(|state| {
        const EVENTTYPE_CLOCK: u8 = wasi::EVENTTYPE_CLOCK.raw();
        const EVENTTYPE_FD_READ: u8 = wasi::EVENTTYPE_FD_READ.raw();
        const EVENTTYPE_FD_WRITE: u8 = wasi::EVENTTYPE_FD_WRITE.raw();

        let mut pollables = Pollables {
            pointer: pollables,
            index: 0,
            length: nsubscriptions,
        };

        for subscription in subscriptions {
            pollables.push(match subscription.u.tag {
                EVENTTYPE_CLOCK => {
                    let clock = &subscription.u.u.clock;
                    let absolute = (clock.flags & SUBCLOCKFLAGS_SUBSCRIPTION_CLOCK_ABSTIME)
                        == SUBCLOCKFLAGS_SUBSCRIPTION_CLOCK_ABSTIME;
                    match clock.id {
                        CLOCKID_REALTIME => {
                            let timeout = if absolute {
                                // Convert `clock.timeout` to `Datetime`.
                                let mut datetime = wall_clock::Datetime {
                                    seconds: clock.timeout / 1_000_000_000,
                                    nanoseconds: (clock.timeout % 1_000_000_000) as _,
                                };

                                // Subtract `now`.
                                let now = wall_clock::now();
                                datetime.seconds -= now.seconds;
                                if datetime.nanoseconds < now.nanoseconds {
                                    datetime.seconds -= 1;
                                    datetime.nanoseconds += 1_000_000_000;
                                }
                                datetime.nanoseconds -= now.nanoseconds;

                                // Convert to nanoseconds.
                                let nanos = datetime
                                    .seconds
                                    .checked_mul(1_000_000_000)
                                    .ok_or(ERRNO_OVERFLOW)?;
                                nanos
                                    .checked_add(datetime.nanoseconds.into())
                                    .ok_or(ERRNO_OVERFLOW)?
                            } else {
                                clock.timeout
                            };

                            monotonic_clock::subscribe_duration(timeout)
                        }

                        CLOCKID_MONOTONIC => {
                            if absolute {
                                monotonic_clock::subscribe_instant(clock.timeout)
                            } else {
                                monotonic_clock::subscribe_duration(clock.timeout)
                            }
                        }

                        _ => return Err(ERRNO_INVAL),
                    }
                }

                EVENTTYPE_FD_READ => state
                    .descriptors()
                    .get_read_stream(subscription.u.u.fd_read.file_descriptor)
                    .map(|stream| stream.subscribe())?,

                EVENTTYPE_FD_WRITE => state
                    .descriptors()
                    .get_write_stream(subscription.u.u.fd_write.file_descriptor)
                    .map(|stream| stream.subscribe())?,

                _ => return Err(ERRNO_INVAL),
            });
        }

        #[link(wasm_import_module = "wasi:io/poll@0.2.0-rc-2023-11-10")]
        #[allow(improper_ctypes)] // FIXME(bytecodealliance/wit-bindgen#684)
        extern "C" {
            #[link_name = "poll"]
            fn poll_import(pollables: *const Pollable, len: usize, rval: *mut ReadyList);
        }
        let mut ready_list = ReadyList {
            base: std::ptr::null(),
            len: 0,
        };

        state.import_alloc.with_buffer(
            results.cast(),
            nsubscriptions
                .checked_mul(size_of::<u32>())
                .trapping_unwrap(),
            || {
                poll_import(
                    pollables.pointer,
                    pollables.length,
                    &mut ready_list as *mut _,
                );
            },
        );

        assert!(ready_list.len <= nsubscriptions);
        assert_eq!(ready_list.base, results as *const u32);

        drop(pollables);

        let ready = std::slice::from_raw_parts(ready_list.base, ready_list.len);

        let mut count = 0;

        for subscription in ready {
            let subscription = *subscriptions.as_ptr().add(*subscription as usize);

            let type_;

            let (error, nbytes, flags) = match subscription.u.tag {
                EVENTTYPE_CLOCK => {
                    type_ = wasi::EVENTTYPE_CLOCK;
                    (ERRNO_SUCCESS, 0, 0)
                }

                EVENTTYPE_FD_READ => {
                    type_ = wasi::EVENTTYPE_FD_READ;
                    let ds = state.descriptors();
                    let desc = ds
                        .get(subscription.u.u.fd_read.file_descriptor)
                        .trapping_unwrap();
                    match desc {
                        Descriptor::Streams(streams) => match &streams.type_ {
                            #[cfg(not(feature = "proxy"))]
                            StreamType::File(file) => match file.fd.stat() {
                                Ok(stat) => {
                                    let nbytes = stat.size.saturating_sub(file.position.get());
                                    (
                                        ERRNO_SUCCESS,
                                        nbytes,
                                        if nbytes == 0 {
                                            EVENTRWFLAGS_FD_READWRITE_HANGUP
                                        } else {
                                            0
                                        },
                                    )
                                }
                                Err(e) => (e.into(), 1, 0),
                            },
                            StreamType::Stdio(_) => (ERRNO_SUCCESS, 1, 0),
                        },
                        _ => unreachable!(),
                    }
                }
                EVENTTYPE_FD_WRITE => {
                    type_ = wasi::EVENTTYPE_FD_WRITE;
                    let ds = state.descriptors();
                    let desc = ds
                        .get(subscription.u.u.fd_write.file_descriptor)
                        .trapping_unwrap();
                    match desc {
                        Descriptor::Streams(streams) => match &streams.type_ {
                            #[cfg(not(feature = "proxy"))]
                            StreamType::File(_) => (ERRNO_SUCCESS, 1, 0),
                            StreamType::Stdio(_) => (ERRNO_SUCCESS, 1, 0),
                        },
                        _ => unreachable!(),
                    }
                }

                _ => unreachable!(),
            };

            *out.add(count) = Event {
                userdata: subscription.userdata,
                error,
                type_,
                fd_readwrite: EventFdReadwrite { nbytes, flags },
            };

            count += 1;
        }

        *nevents = count;

        Ok(())
    })
}

/// Terminate the process normally. An exit code of 0 indicates successful
/// termination of the program. The meanings of other values is dependent on
/// the environment.
#[no_mangle]
pub unsafe extern "C" fn proc_exit(rval: Exitcode) -> ! {
    #[cfg(feature = "proxy")]
    {
        unreachable!("no other implementation available in proxy world");
    }
    #[cfg(not(feature = "proxy"))]
    {
        let status = if rval == 0 { Ok(()) } else { Err(()) };
        crate::bindings::wasi::cli::exit::exit(status); // does not return
        unreachable!("host exit implementation didn't exit!") // actually unreachable
    }
}

/// Send a signal to the process of the calling thread.
/// Note: This is similar to `raise` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn proc_raise(_sig: Signal) -> Errno {
    unreachable!()
}

/// Temporarily yield execution of the calling thread.
/// Note: This is similar to `sched_yield` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn sched_yield() -> Errno {
    // TODO: This is not yet covered in Preview2.

    ERRNO_SUCCESS
}

/// Write high-quality random data into a buffer.
/// This function blocks when the implementation is unable to immediately
/// provide sufficient high-quality random data.
/// This function may execute slowly, so when large mounts of random data are
/// required, it's advisable to use this function to seed a pseudo-random
/// number generator, rather than to provide the random data directly.
#[no_mangle]
pub unsafe extern "C" fn random_get(buf: *mut u8, buf_len: Size) -> Errno {
    if matches!(
        get_allocation_state(),
        AllocationState::StackAllocated | AllocationState::StateAllocated
    ) {
        State::with(|state| {
            assert_eq!(buf_len as u32 as Size, buf_len);
            let result = state
                .import_alloc
                .with_buffer(buf, buf_len, || random::get_random_bytes(buf_len as u64));
            assert_eq!(result.as_ptr(), buf);

            // The returned buffer's memory was allocated in `buf`, so don't separately
            // free it.
            forget(result);

            Ok(())
        })
    } else {
        ERRNO_SUCCESS
    }
}

/// Accept a new incoming connection.
/// Note: This is similar to `accept` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn sock_accept(_fd: Fd, _flags: Fdflags, _connection: *mut Fd) -> Errno {
    unreachable!()
}

/// Receive a message from a socket.
/// Note: This is similar to `recv` in POSIX, though it also supports reading
/// the data into multiple buffers in the manner of `readv`.
#[no_mangle]
pub unsafe extern "C" fn sock_recv(
    _fd: Fd,
    _ri_data_ptr: *const Iovec,
    _ri_data_len: usize,
    _ri_flags: Riflags,
    _ro_datalen: *mut Size,
    _ro_flags: *mut Roflags,
) -> Errno {
    unreachable!()
}

/// Send a message on a socket.
/// Note: This is similar to `send` in POSIX, though it also supports writing
/// the data from multiple buffers in the manner of `writev`.
#[no_mangle]
pub unsafe extern "C" fn sock_send(
    _fd: Fd,
    _si_data_ptr: *const Ciovec,
    _si_data_len: usize,
    _si_flags: Siflags,
    _so_datalen: *mut Size,
) -> Errno {
    unreachable!()
}

/// Shut down socket send and receive channels.
/// Note: This is similar to `shutdown` in POSIX.
#[no_mangle]
pub unsafe extern "C" fn sock_shutdown(_fd: Fd, _how: Sdflags) -> Errno {
    unreachable!()
}

#[cfg(not(feature = "proxy"))]
fn datetime_to_timestamp(datetime: Option<filesystem::Datetime>) -> Timestamp {
    match datetime {
        Some(datetime) => u64::from(datetime.nanoseconds)
            .saturating_add(datetime.seconds.saturating_mul(1_000_000_000)),
        None => 0,
    }
}

#[cfg(not(feature = "proxy"))]
fn at_flags_from_lookupflags(flags: Lookupflags) -> filesystem::PathFlags {
    if flags & LOOKUPFLAGS_SYMLINK_FOLLOW == LOOKUPFLAGS_SYMLINK_FOLLOW {
        filesystem::PathFlags::SYMLINK_FOLLOW
    } else {
        filesystem::PathFlags::empty()
    }
}

#[cfg(not(feature = "proxy"))]
fn o_flags_from_oflags(flags: Oflags) -> filesystem::OpenFlags {
    let mut o_flags = filesystem::OpenFlags::empty();
    if flags & OFLAGS_CREAT == OFLAGS_CREAT {
        o_flags |= filesystem::OpenFlags::CREATE;
    }
    if flags & OFLAGS_DIRECTORY == OFLAGS_DIRECTORY {
        o_flags |= filesystem::OpenFlags::DIRECTORY;
    }
    if flags & OFLAGS_EXCL == OFLAGS_EXCL {
        o_flags |= filesystem::OpenFlags::EXCLUSIVE;
    }
    if flags & OFLAGS_TRUNC == OFLAGS_TRUNC {
        o_flags |= filesystem::OpenFlags::TRUNCATE;
    }
    o_flags
}

#[cfg(not(feature = "proxy"))]
fn descriptor_flags_from_flags(rights: Rights, fdflags: Fdflags) -> filesystem::DescriptorFlags {
    let mut flags = filesystem::DescriptorFlags::empty();
    if rights & wasi::RIGHTS_FD_READ == wasi::RIGHTS_FD_READ {
        flags |= filesystem::DescriptorFlags::READ;
    }
    if rights & wasi::RIGHTS_FD_WRITE == wasi::RIGHTS_FD_WRITE {
        flags |= filesystem::DescriptorFlags::WRITE;
    }
    if fdflags & wasi::FDFLAGS_SYNC == wasi::FDFLAGS_SYNC {
        flags |= filesystem::DescriptorFlags::FILE_INTEGRITY_SYNC;
    }
    if fdflags & wasi::FDFLAGS_DSYNC == wasi::FDFLAGS_DSYNC {
        flags |= filesystem::DescriptorFlags::DATA_INTEGRITY_SYNC;
    }
    if fdflags & wasi::FDFLAGS_RSYNC == wasi::FDFLAGS_RSYNC {
        flags |= filesystem::DescriptorFlags::REQUESTED_WRITE_SYNC;
    }
    flags
}

#[cfg(not(feature = "proxy"))]
impl From<filesystem::ErrorCode> for Errno {
    #[inline(never)] // Disable inlining as this is bulky and relatively cold.
    fn from(err: filesystem::ErrorCode) -> Errno {
        match err {
            // Use a black box to prevent the optimizer from generating a
            // lookup table, which would require a static initializer.
            filesystem::ErrorCode::Access => black_box(ERRNO_ACCES),
            filesystem::ErrorCode::WouldBlock => ERRNO_AGAIN,
            filesystem::ErrorCode::Already => ERRNO_ALREADY,
            filesystem::ErrorCode::BadDescriptor => ERRNO_BADF,
            filesystem::ErrorCode::Busy => ERRNO_BUSY,
            filesystem::ErrorCode::Deadlock => ERRNO_DEADLK,
            filesystem::ErrorCode::Quota => ERRNO_DQUOT,
            filesystem::ErrorCode::Exist => ERRNO_EXIST,
            filesystem::ErrorCode::FileTooLarge => ERRNO_FBIG,
            filesystem::ErrorCode::IllegalByteSequence => ERRNO_ILSEQ,
            filesystem::ErrorCode::InProgress => ERRNO_INPROGRESS,
            filesystem::ErrorCode::Interrupted => ERRNO_INTR,
            filesystem::ErrorCode::Invalid => ERRNO_INVAL,
            filesystem::ErrorCode::Io => ERRNO_IO,
            filesystem::ErrorCode::IsDirectory => ERRNO_ISDIR,
            filesystem::ErrorCode::Loop => ERRNO_LOOP,
            filesystem::ErrorCode::TooManyLinks => ERRNO_MLINK,
            filesystem::ErrorCode::MessageSize => ERRNO_MSGSIZE,
            filesystem::ErrorCode::NameTooLong => ERRNO_NAMETOOLONG,
            filesystem::ErrorCode::NoDevice => ERRNO_NODEV,
            filesystem::ErrorCode::NoEntry => ERRNO_NOENT,
            filesystem::ErrorCode::NoLock => ERRNO_NOLCK,
            filesystem::ErrorCode::InsufficientMemory => ERRNO_NOMEM,
            filesystem::ErrorCode::InsufficientSpace => ERRNO_NOSPC,
            filesystem::ErrorCode::Unsupported => ERRNO_NOTSUP,
            filesystem::ErrorCode::NotDirectory => ERRNO_NOTDIR,
            filesystem::ErrorCode::NotEmpty => ERRNO_NOTEMPTY,
            filesystem::ErrorCode::NotRecoverable => ERRNO_NOTRECOVERABLE,
            filesystem::ErrorCode::NoTty => ERRNO_NOTTY,
            filesystem::ErrorCode::NoSuchDevice => ERRNO_NXIO,
            filesystem::ErrorCode::Overflow => ERRNO_OVERFLOW,
            filesystem::ErrorCode::NotPermitted => ERRNO_PERM,
            filesystem::ErrorCode::Pipe => ERRNO_PIPE,
            filesystem::ErrorCode::ReadOnly => ERRNO_ROFS,
            filesystem::ErrorCode::InvalidSeek => ERRNO_SPIPE,
            filesystem::ErrorCode::TextFileBusy => ERRNO_TXTBSY,
            filesystem::ErrorCode::CrossDevice => ERRNO_XDEV,
        }
    }
}

#[cfg(not(feature = "proxy"))]
impl From<filesystem::DescriptorType> for wasi::Filetype {
    fn from(ty: filesystem::DescriptorType) -> wasi::Filetype {
        match ty {
            filesystem::DescriptorType::RegularFile => FILETYPE_REGULAR_FILE,
            filesystem::DescriptorType::Directory => FILETYPE_DIRECTORY,
            filesystem::DescriptorType::BlockDevice => FILETYPE_BLOCK_DEVICE,
            filesystem::DescriptorType::CharacterDevice => FILETYPE_CHARACTER_DEVICE,
            // preview1 never had a FIFO code.
            filesystem::DescriptorType::Fifo => FILETYPE_UNKNOWN,
            // TODO: Add a way to disginguish between FILETYPE_SOCKET_STREAM and
            // FILETYPE_SOCKET_DGRAM.
            filesystem::DescriptorType::Socket => unreachable!(),
            filesystem::DescriptorType::SymbolicLink => FILETYPE_SYMBOLIC_LINK,
            filesystem::DescriptorType::Unknown => FILETYPE_UNKNOWN,
        }
    }
}

#[derive(Clone, Copy)]
pub enum BlockingMode {
    NonBlocking,
    Blocking,
}

impl BlockingMode {
    // note: these methods must take self, not &self, to avoid rustc creating a constant
    // out of a BlockingMode literal that it places in .romem, creating a data section and
    // breaking our fragile linking scheme
    fn read(
        self,
        input_stream: &streams::InputStream,
        read_len: u64,
    ) -> Result<Vec<u8>, streams::StreamError> {
        match self {
            BlockingMode::NonBlocking => input_stream.read(read_len),
            BlockingMode::Blocking => input_stream.blocking_read(read_len),
        }
    }
    fn write(
        self,
        output_stream: &streams::OutputStream,
        mut bytes: &[u8],
    ) -> Result<usize, Errno> {
        match self {
            BlockingMode::Blocking => {
                let total = bytes.len();
                while !bytes.is_empty() {
                    let len = bytes.len().min(4096);
                    let (chunk, rest) = bytes.split_at(len);
                    bytes = rest;
                    match output_stream.blocking_write_and_flush(chunk) {
                        Ok(()) => {}
                        Err(streams::StreamError::Closed) => return Err(ERRNO_IO),
                        Err(streams::StreamError::LastOperationFailed(e)) => {
                            return Err(stream_error_to_errno(e))
                        }
                    }
                }
                Ok(total)
            }

            BlockingMode::NonBlocking => {
                let permit = match output_stream.check_write() {
                    Ok(n) => n,
                    Err(streams::StreamError::Closed) => 0,
                    Err(streams::StreamError::LastOperationFailed(e)) => {
                        return Err(stream_error_to_errno(e))
                    }
                };

                let len = bytes.len().min(permit as usize);
                if len == 0 {
                    return Ok(0);
                }

                match output_stream.write(&bytes[..len]) {
                    Ok(_) => {}
                    Err(streams::StreamError::Closed) => return Ok(0),
                    Err(streams::StreamError::LastOperationFailed(e)) => {
                        return Err(stream_error_to_errno(e))
                    }
                }

                match output_stream.blocking_flush() {
                    Ok(_) => {}
                    Err(streams::StreamError::Closed) => return Ok(0),
                    Err(streams::StreamError::LastOperationFailed(e)) => {
                        return Err(stream_error_to_errno(e))
                    }
                }

                Ok(len)
            }
        }
    }
}

#[repr(C)]
#[cfg(not(feature = "proxy"))]
pub struct File {
    /// The handle to the preview2 descriptor that this file is referencing.
    fd: filesystem::Descriptor,

    /// The descriptor type, as supplied by filesystem::get_type at opening
    descriptor_type: filesystem::DescriptorType,

    /// The current-position pointer.
    position: Cell<filesystem::Filesize>,

    /// In append mode, all writes append to the file.
    append: bool,

    /// In blocking mode, read and write calls dispatch to blocking_read and
    /// blocking_check_write on the underlying streams. When false, read and write
    /// dispatch to stream's plain read and check_write.
    blocking_mode: BlockingMode,
}

#[cfg(not(feature = "proxy"))]
impl File {
    fn is_dir(&self) -> bool {
        match self.descriptor_type {
            filesystem::DescriptorType::Directory => true,
            _ => false,
        }
    }
}

const PAGE_SIZE: usize = 65536;

/// The maximum path length. WASI doesn't explicitly guarantee this, but all
/// popular OS's have a `PATH_MAX` of at most 4096, so that's enough for this
/// polyfill.
const PATH_MAX: usize = 4096;

/// Maximum number of bytes to cache for a `wasi::Dirent` plus its path name.
const DIRENT_CACHE: usize = 256;

/// A canary value to detect memory corruption within `State`.
const MAGIC: u32 = u32::from_le_bytes(*b"ugh!");

#[repr(C)] // used for now to keep magic1 and magic2 at the start and end
struct State {
    /// A canary constant value located at the beginning of this structure to
    /// try to catch memory corruption coming from the bottom.
    magic1: u32,

    /// Used to coordinate allocations of `cabi_import_realloc`
    import_alloc: ImportAlloc,

    /// Storage of mapping from preview1 file descriptors to preview2 file
    /// descriptors.
    ///
    /// Do not use this member directly - use State::descriptors() to ensure
    /// lazy initialization happens.
    descriptors: RefCell<Option<Descriptors>>,

    /// Auxiliary storage to handle the `path_readlink` function.
    #[cfg(not(feature = "proxy"))]
    path_buf: UnsafeCell<MaybeUninit<[u8; PATH_MAX]>>,

    /// Long-lived bump allocated memory arena.
    ///
    /// This is used for the cabi_export_realloc to allocate data passed to the
    /// `run` entrypoint. Allocations in this arena are safe to use for
    /// the lifetime of the State struct. It may also be used for import allocations
    /// which need to be long-lived, by using `import_alloc.with_arena`.
    long_lived_arena: BumpArena,

    /// Arguments. Initialized lazily. Access with `State::get_args` to take care of
    /// initialization.
    #[cfg(not(feature = "proxy"))]
    args: Cell<Option<&'static [WasmStr]>>,

    /// Environment variables. Initialized lazily. Access with `State::get_environment`
    /// to take care of initialization.
    #[cfg(not(feature = "proxy"))]
    env_vars: Cell<Option<&'static [StrTuple]>>,

    /// Cache for the `fd_readdir` call for a final `wasi::Dirent` plus path
    /// name that didn't fit into the caller's buffer.
    #[cfg(not(feature = "proxy"))]
    dirent_cache: DirentCache,

    /// The string `..` for use by the directory iterator.
    #[cfg(not(feature = "proxy"))]
    dotdot: [UnsafeCell<u8>; 2],

    /// Another canary constant located at the end of the structure to catch
    /// memory corruption coming from the bottom.
    magic2: u32,
}

#[cfg(not(feature = "proxy"))]
struct DirentCache {
    stream: Cell<Option<DirectoryEntryStream>>,
    for_fd: Cell<wasi::Fd>,
    cookie: Cell<wasi::Dircookie>,
    cached_dirent: Cell<wasi::Dirent>,
    path_data: UnsafeCell<MaybeUninit<[u8; DIRENT_CACHE]>>,
}

#[cfg(not(feature = "proxy"))]
struct DirectoryEntryStream(filesystem::DirectoryEntryStream);

#[repr(C)]
pub struct WasmStr {
    ptr: *const u8,
    len: usize,
}

#[repr(C)]
pub struct WasmStrList {
    base: *const WasmStr,
    len: usize,
}

#[repr(C)]
pub struct StrTuple {
    key: WasmStr,
    value: WasmStr,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct StrTupleList {
    base: *const StrTuple,
    len: usize,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ReadyList {
    base: *const u32,
    len: usize,
}

const fn bump_arena_size() -> usize {
    // The total size of the struct should be a page, so start there
    let mut start = PAGE_SIZE;

    // Remove big chunks of the struct for its various fields.
    start -= size_of::<Descriptors>();
    #[cfg(not(feature = "proxy"))]
    {
        start -= PATH_MAX;
        start -= size_of::<DirentCache>();
    }

    // Remove miscellaneous metadata also stored in state.
    let misc = if cfg!(feature = "proxy") { 7 } else { 14 };
    start -= misc * size_of::<usize>();

    // Everything else is the `command_data` allocation.
    start
}

// Statically assert that the `State` structure is the size of a wasm page. This
// mostly guarantees that it's not larger than one page which is relied upon
// below.
#[cfg(target_arch = "wasm32")]
const _: () = {
    let _size_assert: [(); PAGE_SIZE] = [(); size_of::<State>()];
};

#[allow(unused)]
#[repr(i32)]
enum AllocationState {
    StackUnallocated,
    StackAllocating,
    StackAllocated,
    StateAllocating,
    StateAllocated,
}

#[allow(improper_ctypes)]
extern "C" {
    fn get_state_ptr() -> *mut State;
    fn set_state_ptr(state: *mut State);
    fn get_allocation_state() -> AllocationState;
    fn set_allocation_state(state: AllocationState);
}

impl State {
    fn with(f: impl FnOnce(&State) -> Result<(), Errno>) -> Errno {
        let state_ref = State::ptr();
        assert_eq!(state_ref.magic1, MAGIC);
        assert_eq!(state_ref.magic2, MAGIC);
        let ret = f(state_ref);
        match ret {
            Ok(()) => ERRNO_SUCCESS,
            Err(err) => err,
        }
    }

    fn ptr() -> &'static State {
        unsafe {
            let mut ptr = get_state_ptr();
            if ptr.is_null() {
                ptr = State::new();
                set_state_ptr(ptr);
            }
            &*ptr
        }
    }

    #[cold]
    fn new() -> *mut State {
        #[link(wasm_import_module = "__main_module__")]
        extern "C" {
            fn cabi_realloc(
                old_ptr: *mut u8,
                old_len: usize,
                align: usize,
                new_len: usize,
            ) -> *mut u8;
        }

        assert!(matches!(
            unsafe { get_allocation_state() },
            AllocationState::StackAllocated
        ));

        unsafe { set_allocation_state(AllocationState::StateAllocating) };

        let ret = unsafe {
            cabi_realloc(
                ptr::null_mut(),
                0,
                mem::align_of::<UnsafeCell<State>>(),
                mem::size_of::<UnsafeCell<State>>(),
            ) as *mut State
        };

        unsafe { set_allocation_state(AllocationState::StateAllocated) };

        unsafe {
            Self::init(ret);
        }

        ret
    }

    #[cold]
    unsafe fn init(state: *mut State) {
        state.write(State {
            magic1: MAGIC,
            magic2: MAGIC,
            import_alloc: ImportAlloc::new(),
            descriptors: RefCell::new(None),
            #[cfg(not(feature = "proxy"))]
            path_buf: UnsafeCell::new(MaybeUninit::uninit()),
            long_lived_arena: BumpArena::new(),
            #[cfg(not(feature = "proxy"))]
            args: Cell::new(None),
            #[cfg(not(feature = "proxy"))]
            env_vars: Cell::new(None),
            #[cfg(not(feature = "proxy"))]
            dirent_cache: DirentCache {
                stream: Cell::new(None),
                for_fd: Cell::new(0),
                cookie: Cell::new(wasi::DIRCOOKIE_START),
                cached_dirent: Cell::new(wasi::Dirent {
                    d_next: 0,
                    d_ino: 0,
                    d_type: FILETYPE_UNKNOWN,
                    d_namlen: 0,
                }),
                path_data: UnsafeCell::new(MaybeUninit::uninit()),
            },
            #[cfg(not(feature = "proxy"))]
            dotdot: [UnsafeCell::new(b'.'), UnsafeCell::new(b'.')],
        });
    }

    /// Accessor for the descriptors member that ensures it is properly initialized
    fn descriptors<'a>(&'a self) -> impl Deref<Target = Descriptors> + 'a {
        let mut d = self
            .descriptors
            .try_borrow_mut()
            .unwrap_or_else(|_| unreachable!());
        if d.is_none() {
            *d = Some(Descriptors::new(&self.import_alloc, &self.long_lived_arena));
        }
        RefMut::map(d, |d| d.as_mut().unwrap_or_else(|| unreachable!()))
    }

    /// Mut accessor for the descriptors member that ensures it is properly initialized
    fn descriptors_mut<'a>(&'a self) -> impl DerefMut + Deref<Target = Descriptors> + 'a {
        let mut d = self
            .descriptors
            .try_borrow_mut()
            .unwrap_or_else(|_| unreachable!());
        if d.is_none() {
            *d = Some(Descriptors::new(&self.import_alloc, &self.long_lived_arena));
        }
        RefMut::map(d, |d| d.as_mut().unwrap_or_else(|| unreachable!()))
    }

    #[cfg(not(feature = "proxy"))]
    fn get_environment(&self) -> &[StrTuple] {
        if self.env_vars.get().is_none() {
            #[link(wasm_import_module = "wasi:cli/environment@0.2.0-rc-2023-12-05")]
            extern "C" {
                #[link_name = "get-environment"]
                fn get_environment_import(rval: *mut StrTupleList);
            }
            let mut list = StrTupleList {
                base: std::ptr::null(),
                len: 0,
            };
            self.import_alloc
                .with_arena(&self.long_lived_arena, || unsafe {
                    get_environment_import(&mut list as *mut _)
                });
            self.env_vars.set(Some(unsafe {
                /* allocation comes from long lived arena, so it is safe to
                 * cast this to a &'static slice: */
                std::slice::from_raw_parts(list.base, list.len)
            }));
        }
        self.env_vars.get().trapping_unwrap()
    }

    #[cfg(not(feature = "proxy"))]
    fn get_args(&self) -> &[WasmStr] {
        if self.args.get().is_none() {
            #[link(wasm_import_module = "wasi:cli/environment@0.2.0-rc-2023-12-05")]
            extern "C" {
                #[link_name = "get-arguments"]
                fn get_args_import(rval: *mut WasmStrList);
            }
            let mut list = WasmStrList {
                base: std::ptr::null(),
                len: 0,
            };
            self.import_alloc
                .with_arena(&self.long_lived_arena, || unsafe {
                    get_args_import(&mut list as *mut _)
                });
            self.args.set(Some(unsafe {
                /* allocation comes from long lived arena, so it is safe to
                 * cast this to a &'static slice: */
                std::slice::from_raw_parts(list.base, list.len)
            }));
        }
        self.args.get().trapping_unwrap()
    }
}
