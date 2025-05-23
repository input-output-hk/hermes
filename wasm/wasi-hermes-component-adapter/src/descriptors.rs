use core::{
    cell::{Cell, OnceCell, UnsafeCell},
    mem::MaybeUninit,
};

use wasi::{Errno, Fd};

#[cfg(not(feature = "proxy"))]
use crate::bindings::wasi::filesystem::types as filesystem;
#[cfg(not(feature = "proxy"))]
use crate::File;
use crate::{
    bindings::wasi::{
        cli::{stderr, stdin, stdout},
        io::streams::{InputStream, OutputStream},
    },
    BlockingMode, BumpArena, ImportAlloc, TrappingUnwrap, WasmStr,
};

#[allow(clippy::missing_docs_in_private_items)]
pub const MAX_DESCRIPTORS: usize = 128;

#[repr(C)]
#[allow(clippy::missing_docs_in_private_items)]
pub enum Descriptor {
    /// A closed descriptor, holding a reference to the previous closed
    /// descriptor to support reusing them.
    Closed(Option<Fd>),

    /// Input and/or output wasi-streams, along with stream metadata.
    Streams(Streams),

    Bad,
}

/// Input and/or output wasi-streams, along with a stream type that
/// identifies what kind of stream they are and possibly supporting
/// type-specific operations like seeking.
pub struct Streams {
    /// The input stream, if present.
    pub input: OnceCell<InputStream>,

    /// The output stream, if present.
    pub output: OnceCell<OutputStream>,

    /// Information about the source of the stream.
    pub type_: StreamType,
}

impl Streams {
    /// Return the input stream, initializing it on the fly if needed.
    #[allow(clippy::single_match_else)]
    pub fn get_read_stream(&self) -> Result<&InputStream, Errno> {
        match self.input.get() {
            Some(wasi_stream) => Ok(wasi_stream),
            None => {
                let input = match &self.type_ {
                    // For directories, preview 1 behavior was to return ERRNO_BADF on attempts to
                    // read or write.
                    #[cfg(not(feature = "proxy"))]
                    StreamType::File(File {
                        descriptor_type: filesystem::DescriptorType::Directory,
                        ..
                    }) => return Err(wasi::ERRNO_BADF),
                    // For files, we may have adjusted the position for seeking, so
                    // create a new stream.
                    #[cfg(not(feature = "proxy"))]
                    StreamType::File(file) => file.fd.read_via_stream(file.position.get())?,
                    _ => return Err(wasi::ERRNO_BADF),
                };
                self.input.set(input).trapping_unwrap();
                Ok(self.input.get().trapping_unwrap())
            },
        }
    }

    /// Return the output stream, initializing it on the fly if needed.
    #[allow(clippy::single_match_else)]
    pub fn get_write_stream(&self) -> Result<&OutputStream, Errno> {
        match self.output.get() {
            Some(wasi_stream) => Ok(wasi_stream),
            None => {
                let output = match &self.type_ {
                    // For directories, preview 1 behavior was to return ERRNO_BADF on attempts to
                    // read or write.
                    #[cfg(not(feature = "proxy"))]
                    StreamType::File(File {
                        descriptor_type: filesystem::DescriptorType::Directory,
                        ..
                    }) => return Err(wasi::ERRNO_BADF),
                    // For files, we may have adjusted the position for seeking, so
                    // create a new stream.
                    #[cfg(not(feature = "proxy"))]
                    StreamType::File(file) => {
                        if file.append {
                            file.fd.append_via_stream()?
                        } else {
                            file.fd.write_via_stream(file.position.get())?
                        }
                    },
                    _ => return Err(wasi::ERRNO_BADF),
                };
                self.output.set(output).trapping_unwrap();
                Ok(self.output.get().trapping_unwrap())
            },
        }
    }
}

#[allow(clippy::missing_docs_in_private_items)]
pub enum StreamType {
    /// Streams for implementing stdio.
    Stdio(Stdio),

    /// Streaming data with a file.
    #[cfg(not(feature = "proxy"))]
    File(File),
}

#[allow(clippy::missing_docs_in_private_items)]
pub enum Stdio {
    Stdin,
    Stdout,
    Stderr,
}

impl Stdio {
    #[allow(clippy::missing_docs_in_private_items, clippy::unused_self)]
    pub fn filetype(&self) -> wasi::Filetype {
        // `self` is unused in this simplified version of the function.
        // We retain it for internal API compatibility.
        // wasi::FILETYPE_CHARACTER_DEVICE
        wasi::FILETYPE_UNKNOWN
    }
}

#[repr(C)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct Descriptors {
    /// Storage of mapping from preview1 file descriptors to preview2 file
    /// descriptors.
    table: UnsafeCell<MaybeUninit<[Descriptor; MAX_DESCRIPTORS]>>,
    table_len: Cell<u16>,

    /// Points to the head of a free-list of closed file descriptors.
    closed: Option<Fd>,

    /// Preopened directories. Initialized lazily. Access with `State::get_preopens`
    /// to take care of initialization.
    #[cfg(not(feature = "proxy"))]
    preopens: Cell<Option<&'static [Preopen]>>,
}

#[allow(clippy::missing_docs_in_private_items)]
impl Descriptors {
    #[allow(clippy::items_after_statements)]
    pub fn new(import_alloc: &ImportAlloc, arena: &BumpArena) -> Self {
        let d = Descriptors {
            table: UnsafeCell::new(MaybeUninit::uninit()),
            table_len: Cell::new(0),
            closed: None,
            #[cfg(not(feature = "proxy"))]
            preopens: Cell::new(None),
        };

        fn new_once<T>(val: T) -> OnceCell<T> {
            let cell = OnceCell::new();
            let _unused = cell.set(val);
            cell
        }

        d.push(Descriptor::Streams(Streams {
            input: new_once(stdin::get_stdin()),
            output: OnceCell::new(),
            type_: StreamType::Stdio(Stdio::Stdin),
        }))
        .trapping_unwrap();
        d.push(Descriptor::Streams(Streams {
            input: OnceCell::new(),
            output: new_once(stdout::get_stdout()),
            type_: StreamType::Stdio(Stdio::Stdout),
        }))
        .trapping_unwrap();
        d.push(Descriptor::Streams(Streams {
            input: OnceCell::new(),
            output: new_once(stderr::get_stderr()),
            type_: StreamType::Stdio(Stdio::Stderr),
        }))
        .trapping_unwrap();

        #[cfg(not(feature = "proxy"))]
        d.open_preopens(import_alloc, arena);
        d
    }

    #[cfg(not(feature = "proxy"))]
    #[allow(trivial_casts)]
    #[allow(clippy::borrow_as_ptr)]
    #[allow(clippy::semicolon_if_nothing_returned)]
    fn open_preopens(&self, import_alloc: &ImportAlloc, arena: &BumpArena) {
        // This should not be preopnes@0.2.0 for wasi
        // #[link(wasm_import_module = "wasi:filesystem/preopens@0.2.0-rc-2023-11-10")]
        #[link(wasm_import_module = "wasi:filesystem/preopens@0.2.0")]
        #[allow(improper_ctypes)] // FIXME(bytecodealliance/wit-bindgen#684)
        extern "C" {
            #[link_name = "get-directories"]
            fn get_preopens_import(rval: *mut PreopenList);
        }
        let mut list = PreopenList {
            base: std::ptr::null(),
            len: 0,
        };
        import_alloc.with_arena(arena, || unsafe {
            get_preopens_import(std::ptr::from_mut(&mut list))
        });
        let preopens: &'static [Preopen] = unsafe {
            // allocation comes from long lived arena, so it is safe to
            // cast this to a &'static slice:
            std::slice::from_raw_parts(list.base, list.len)
        };
        for preopen in preopens {
            // Acquire ownership of the descriptor, leaving the rest of the
            // `Preopen` struct in place.
            let descriptor = unsafe { preopen.descriptor.assume_init_read() };
            // Expectation is that the descriptor index is initialized with
            // stdio (0,1,2) and no others, so that preopens are 3..
            let descriptor_type = descriptor.get_type().trapping_unwrap();
            self.push(Descriptor::Streams(Streams {
                input: OnceCell::new(),
                output: OnceCell::new(),
                type_: StreamType::File(File {
                    fd: descriptor,
                    descriptor_type,
                    position: Cell::new(0),
                    append: false,
                    blocking_mode: BlockingMode::Blocking,
                }),
            }))
            .trapping_unwrap();
        }

        self.preopens.set(Some(preopens));
    }

    #[allow(clippy::indexing_slicing)]
    #[allow(clippy::unnecessary_fallible_conversions)]
    fn push(&self, desc: Descriptor) -> Result<Fd, Errno> {
        unsafe {
            let table = (*self.table.get()).as_mut_ptr();
            let len = usize::try_from(self.table_len.get()).trapping_unwrap();
            if len >= (*table).len() {
                return Err(wasi::ERRNO_NOMEM);
            }
            core::ptr::addr_of_mut!((*table)[len]).write(desc);
            self.table_len.set(u16::try_from(len + 1).trapping_unwrap());
            Ok(Fd::from(u32::try_from(len).trapping_unwrap()))
        }
    }

    #[allow(clippy::unnecessary_fallible_conversions)]
    fn table(&self) -> &[Descriptor] {
        unsafe {
            std::slice::from_raw_parts(
                (*self.table.get()).as_ptr().cast(),
                usize::try_from(self.table_len.get()).trapping_unwrap(),
            )
        }
    }

    #[allow(clippy::unnecessary_fallible_conversions)]
    fn table_mut(&mut self) -> &mut [Descriptor] {
        unsafe {
            std::slice::from_raw_parts_mut(
                (*self.table.get()).as_mut_ptr().cast(),
                usize::try_from(self.table_len.get()).trapping_unwrap(),
            )
        }
    }

    #[allow(clippy::unreachable)]
    #[allow(clippy::single_match_else)]
    pub fn open(&mut self, d: Descriptor) -> Result<Fd, Errno> {
        match self.closed {
            // No closed descriptors: expand table
            None => self.push(d),
            Some(freelist_head) => {
                // Pop an item off the freelist
                let freelist_desc = self.get_mut(freelist_head).trapping_unwrap();
                let next_closed = match freelist_desc {
                    Descriptor::Closed(next) => *next,
                    _ => unreachable!("impossible: freelist points to a closed descriptor"),
                };
                // Write descriptor to the entry at the head of the list
                *freelist_desc = d;
                // Point closed to the following item
                self.closed = next_closed;
                Ok(freelist_head)
            },
        }
    }

    pub fn get(&self, fd: Fd) -> Result<&Descriptor, Errno> {
        self.table()
            .get(usize::try_from(fd).trapping_unwrap())
            .ok_or(wasi::ERRNO_BADF)
    }

    pub fn get_mut(&mut self, fd: Fd) -> Result<&mut Descriptor, Errno> {
        self.table_mut()
            .get_mut(usize::try_from(fd).trapping_unwrap())
            .ok_or(wasi::ERRNO_BADF)
    }

    #[cfg(not(feature = "proxy"))]
    pub fn get_preopen(&self, fd: Fd) -> Option<&Preopen> {
        let preopens = self.preopens.get().trapping_unwrap();
        // Subtract 3 for the stdio indices to compute the preopen index.
        let index = fd.checked_sub(3)? as usize;
        preopens.get(index)
    }

    // Internal: close a fd, returning the descriptor.
    #[allow(clippy::single_match)]
    fn close_(&mut self, fd: Fd) -> Result<Descriptor, Errno> {
        // Throw an error if closing an fd which is already closed
        match self.get(fd)? {
            Descriptor::Closed(_) => Err(wasi::ERRNO_BADF)?,
            _ => {},
        }
        // Mutate the descriptor to be closed, and push the closed fd onto the head of the linked
        // list:
        let last_closed = self.closed;
        let prev = std::mem::replace(self.get_mut(fd)?, Descriptor::Closed(last_closed));
        self.closed = Some(fd);
        Ok(prev)
    }

    // Close an fd.
    pub fn close(&mut self, fd: Fd) -> Result<(), Errno> {
        drop(self.close_(fd)?);
        Ok(())
    }

    // Expand the table by pushing a closed descriptor to the end. Used for renumbering.
    fn push_closed(&mut self) -> Result<(), Errno> {
        let old_closed = self.closed;
        let new_closed = self.push(Descriptor::Closed(old_closed))?;
        self.closed = Some(new_closed);
        Ok(())
    }

    // Implementation of fd_renumber
    #[allow(clippy::cast_lossless)]
    pub fn renumber(&mut self, from_fd: Fd, to_fd: Fd) -> Result<(), Errno> {
        // First, ensure from_fd is in bounds:
        let _ = self.get(from_fd)?;
        // Expand table until to_fd is in bounds as well:
        while self.table_len.get() as u32 <= to_fd {
            self.push_closed()?;
        }
        // Then, close from_fd and put its contents into to_fd:
        let desc = self.close_(from_fd)?;
        // TODO FIXME if this overwrites a preopen, do we need to clear it from the preopen table?
        *self.get_mut(to_fd)? = desc;

        Ok(())
    }

    // A bunch of helper functions implemented in terms of the above pub functions:

    pub fn get_stream_with_error_mut(
        &mut self, fd: Fd, error: Errno,
    ) -> Result<&mut Streams, Errno> {
        match self.get_mut(fd)? {
            Descriptor::Streams(streams) => Ok(streams),
            Descriptor::Closed(_) | Descriptor::Bad => Err(error),
        }
    }

    #[cfg(not(feature = "proxy"))]
    #[allow(clippy::match_same_arms)]
    pub fn get_file_with_error(&self, fd: Fd, error: Errno) -> Result<&File, Errno> {
        match self.get(fd)? {
            Descriptor::Streams(Streams {
                type_:
                    StreamType::File(File {
                        descriptor_type: filesystem::DescriptorType::Directory,
                        ..
                    }),
                ..
            }) => Err(wasi::ERRNO_BADF),
            Descriptor::Streams(Streams {
                type_: StreamType::File(file),
                ..
            }) => Ok(file),
            Descriptor::Closed(_) => Err(wasi::ERRNO_BADF),
            _ => Err(error),
        }
    }

    #[cfg(not(feature = "proxy"))]
    pub fn get_file(&self, fd: Fd) -> Result<&File, Errno> {
        self.get_file_with_error(fd, wasi::ERRNO_INVAL)
    }

    #[cfg(not(feature = "proxy"))]
    pub fn get_dir(&self, fd: Fd) -> Result<&File, Errno> {
        match self.get(fd)? {
            Descriptor::Streams(Streams {
                type_:
                    StreamType::File(
                        file @ File {
                            descriptor_type: filesystem::DescriptorType::Directory,
                            ..
                        },
                    ),
                ..
            }) => Ok(file),
            Descriptor::Streams(Streams {
                type_: StreamType::File(File { .. }),
                ..
            }) => Err(wasi::ERRNO_NOTDIR),
            _ => Err(wasi::ERRNO_BADF),
        }
    }

    #[cfg(not(feature = "proxy"))]
    pub fn get_seekable_file(&self, fd: Fd) -> Result<&File, Errno> {
        self.get_file_with_error(fd, wasi::ERRNO_SPIPE)
    }

    pub fn get_seekable_stream_mut(&mut self, fd: Fd) -> Result<&mut Streams, Errno> {
        self.get_stream_with_error_mut(fd, wasi::ERRNO_SPIPE)
    }

    // pub fn get_read_stream(&self, fd: Fd) -> Result<&InputStream, Errno> {
    // match self.get(fd)? {
    // Descriptor::Streams(streams) => streams.get_read_stream(),
    // Descriptor::Closed(_) | Descriptor::Bad => Err(wasi::ERRNO_BADF),
    // }
    // }
    //
    // pub fn get_write_stream(&self, fd: Fd) -> Result<&OutputStream, Errno> {
    // match self.get(fd)? {
    // Descriptor::Streams(streams) => streams.get_write_stream(),
    // Descriptor::Closed(_) | Descriptor::Bad => Err(wasi::ERRNO_BADF),
    // }
    // }
}

#[cfg(not(feature = "proxy"))]
#[repr(C)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct Preopen {
    /// This is `MaybeUninit` because we take ownership of the `Descriptor` to
    /// put it in our own table.
    pub descriptor: MaybeUninit<filesystem::Descriptor>,
    pub path: WasmStr,
}

#[cfg(not(feature = "proxy"))]
#[repr(C)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct PreopenList {
    pub base: *const Preopen,
    pub len: usize,
}
