use crate::progress::InternalProgress;
use std::io::{ErrorKind, Read};
use std::sync::{Arc, Mutex};
use tracing::{trace, warn};

pub struct DataChunk {
    pub chunk_no: usize,
    pub data: Vec<u8>,
    pub range: std::ops::Range<usize>,
}

pub struct MpscReaderFromReceiver {
    pos: usize,
    receiver: std::sync::mpsc::Receiver<DataChunk>,
    current_chunk_no: usize,
    current_buf: Vec<u8>,
    current_buf_pos: usize,
    chunk_waiting_list: Vec<DataChunk>,
    debug: bool,
    is_unpack: bool,
    progress_context: Arc<Mutex<InternalProgress>>,
}

impl MpscReaderFromReceiver {
    pub fn new(
        receiver: std::sync::mpsc::Receiver<DataChunk>, debug: bool,
        progress_context: Arc<Mutex<InternalProgress>>, is_unpack: bool,
    ) -> Self {
        Self {
            pos: 0,
            current_chunk_no: 0,
            receiver,
            current_buf: Vec::new(),
            current_buf_pos: 0,
            chunk_waiting_list: Vec::new(),
            debug,
            progress_context,
            is_unpack,
        }
    }
}

impl Read for MpscReaderFromReceiver {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let starting_pos = self.pos;
        let found_chunk =
            if self.current_buf.is_empty() || self.current_buf_pos >= self.current_buf.len() {
                let mut found_idx = None;
                for (idx, chunk) in self.chunk_waiting_list.iter().enumerate() {
                    if chunk.range.start == self.pos {
                        if self.debug {
                            warn!("Found compatible chunk from waiting list {}", self.pos);
                        }
                        found_idx = Some(idx);
                        break;
                    }
                }
                if let Some(found_idx) = found_idx {
                    let dt = self.chunk_waiting_list.swap_remove(found_idx);
                    Some(dt)
                } else {
                    loop {
                        let new_chunk = self.receiver.recv().map_err(|err| {
                            std::io::Error::new(
                                ErrorKind::InvalidData,
                                format!("Receive error {err:?}"),
                            )
                        })?;
                        if new_chunk.range.start == self.pos {
                            if self.debug {
                                warn!("Found compatible chunk {}", self.pos);
                            }
                            break Some(new_chunk);
                        } else {
                            if self.debug {
                                warn!(
                                    "Found incompatible chunk, adding to waiting list {}",
                                    new_chunk.range.start
                                );
                            }
                            self.chunk_waiting_list.push(new_chunk);
                        }
                    }
                }
            } else {
                None
            };
        if let Some(found_chunk) = found_chunk {
            self.current_chunk_no = found_chunk.chunk_no;
            self.current_buf = found_chunk.data;
            self.current_buf_pos = 0;
            if self.is_unpack {
                //keep chunk hisotry
                let chunk_history = 1;
                if self.current_chunk_no >= chunk_history {
                    let mut pc = self.progress_context.lock().unwrap();
                    pc.current_chunks
                        .remove(&(self.current_chunk_no - chunk_history));
                }
            }
        } else {
        }

        let min_val = std::cmp::min(self.current_buf.len() - self.current_buf_pos, buf.len());

        let src_slice = &self.current_buf[self.current_buf_pos..(self.current_buf_pos + min_val)];
        buf[0..min_val].copy_from_slice(src_slice);
        self.current_buf_pos += min_val;
        self.pos += min_val;

        if self.is_unpack {
            let mut pc = self.progress_context.lock().unwrap();

            if let Some(chunk) = pc.current_chunks.get_mut(&self.current_chunk_no) {
                chunk.unpacked = self.current_buf_pos;
            }
        }
        trace!(
            "Chunk read: starting_pos: {} / length: {}",
            starting_pos,
            self.pos - starting_pos
        );
        Ok(min_val)
    }
}
