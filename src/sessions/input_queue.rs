//! Ordered PTY input. Producers never wait for the child to consume input.
use std::{
    collections::VecDeque,
    sync::{Condvar, Mutex},
};

pub(super) const INPUT_LIMIT: usize = 1024 * 1024;
const PENDING_LIMIT: usize = 4 * INPUT_LIMIT;
const WRITE_BLOCK: usize = 8192;

pub(super) struct Batch {
    pub bytes: Vec<u8>,
    sequence: u64,
}

#[derive(Default)]
struct State {
    queue: VecDeque<Batch>,
    // Includes the batch currently held by the writer.
    pending: usize,
    sequence: u64,
    warning: Option<(u64, &'static str)>,
    closed: bool,
}

#[derive(Default)]
pub(super) struct InputQueue {
    state: Mutex<State>,
    ready: Condvar,
}

impl InputQueue {
    /// Admission is all-or-nothing, before copying any of the input.
    pub fn push(&self, bytes: &[u8]) -> Result<(), ()> {
        let mut state = self.state.lock().unwrap();
        if state.closed || bytes.is_empty() {
            return Ok(());
        }
        state.sequence += 1;
        let sequence = state.sequence;
        let warning = if bytes.len() > INPUT_LIMIT {
            Some("Input not sent: paste exceeds 1 MiB. Use a smaller paste.")
        } else if bytes.len() > PENDING_LIMIT - state.pending {
            Some(
                "Input not sent: terminal input buffer is full. Retry when the program resumes reading.",
            )
        } else {
            None
        };
        if let Some(warning) = warning {
            state.warning = Some((sequence, warning));
            return Err(());
        }
        state.pending += bytes.len();
        let mut remaining = bytes;
        if let Some(tail) = state.queue.back_mut() {
            let count = remaining.len().min(WRITE_BLOCK - tail.bytes.len());
            if count > 0 {
                tail.bytes.extend_from_slice(&remaining[..count]);
                tail.sequence = sequence;
                remaining = &remaining[count..];
            }
        }
        for chunk in remaining.chunks(WRITE_BLOCK) {
            state.queue.push_back(Batch {
                bytes: chunk.to_vec(),
                sequence,
            });
        }
        self.ready.notify_one();
        Ok(())
    }

    pub fn next(&self) -> Option<Batch> {
        let mut state = self.state.lock().unwrap();
        loop {
            if state.closed {
                return None;
            }
            if let Some(batch) = state.queue.pop_front() {
                return Some(batch);
            }
            state = self.ready.wait(state).unwrap();
        }
    }

    /// Clear only warnings older than a successfully delivered input. An
    /// already in-flight write cannot erase a more recent rejection.
    pub fn delivered(&self, batch: &Batch) -> bool {
        let mut state = self.state.lock().unwrap();
        state.pending = state.pending.saturating_sub(batch.bytes.len());
        if state
            .warning
            .is_some_and(|(sequence, _)| batch.sequence > sequence)
        {
            state.warning = None;
            return true;
        }
        false
    }

    pub fn warning(&self) -> Option<String> {
        self.state
            .lock()
            .unwrap()
            .warning
            .map(|(_, text)| text.into())
    }

    pub fn close(&self) {
        let mut state = self.state.lock().unwrap();
        state.closed = true;
        state.queue.clear();
        state.pending = 0;
        state.warning = None;
        self.ready.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_event_bursts_keep_every_byte_in_order() {
        let queue = InputQueue::default();
        let expected: Vec<u8> = (0..20000).map(|i| (i % 251) as u8).collect();
        for byte in &expected {
            assert!(queue.push(&[*byte]).is_ok());
        }
        let mut received = Vec::new();
        while received.len() < expected.len() {
            let batch = queue.next().unwrap();
            received.extend_from_slice(&batch.bytes);
            queue.delivered(&batch);
        }
        assert_eq!(received, expected);
        assert_eq!(queue.warning(), None);
    }

    #[test]
    fn stalled_writer_is_bounded_and_rejection_recovers_only_after_new_input_is_delivered() {
        let queue = InputQueue::default();
        for _ in 0..4 {
            assert!(queue.push(&vec![b'x'; INPUT_LIMIT]).is_ok());
        }
        let in_flight = queue.next().unwrap();
        // Removing a batch from the queue does not free the in-flight budget.
        assert!(queue.push(b"rejected").is_err());
        assert!(queue.warning().unwrap().contains("buffer is full"));
        assert!(!queue.delivered(&in_flight));
        assert!(queue.warning().is_some());
        assert!(queue.push(b"retry").is_ok());
        let mut received = in_flight.bytes.clone();
        while received.len() < PENDING_LIMIT + 5 {
            let batch = queue.next().unwrap();
            received.extend_from_slice(&batch.bytes);
            queue.delivered(&batch);
            if received.len() <= PENDING_LIMIT {
                assert!(queue.warning().is_some());
            }
        }
        assert_eq!(&received[..PENDING_LIMIT], vec![b'x'; PENDING_LIMIT]);
        assert_eq!(&received[PENDING_LIMIT..], b"retry");
        assert_eq!(queue.warning(), None);
    }

    #[test]
    fn oversized_paste_is_atomic_and_close_wakes_waiting_writer() {
        let queue = std::sync::Arc::new(InputQueue::default());
        assert!(queue.push(&vec![b'x'; INPUT_LIMIT + 1]).is_err());
        assert_eq!(queue.state.lock().unwrap().pending, 0);
        let reader = queue.clone();
        let worker = std::thread::spawn(move || assert!(reader.next().is_none()));
        queue.close();
        worker.join().unwrap();
        assert!(queue.push(b"after close").is_ok());
        assert!(queue.next().is_none());
    }
}
