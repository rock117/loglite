use chrono::Utc;
use std::sync::atomic::{AtomicI64, AtomicU16, Ordering};

/// A minimal Snowflake-style ID generator.
///
/// This generator produces roughly time-ordered 64-bit integers suitable as primary keys.
/// It is configured by `LOGLITE_NODE_ID` to avoid collisions when multiple instances exist.
pub struct Snowflake {
    node_id: i64,
    last_ms: AtomicI64,
    seq: AtomicU16,
}

impl Snowflake {
    /// Create a new generator.
    pub fn new(node_id: i64) -> Self {
        Self {
            node_id,
            last_ms: AtomicI64::new(0),
            seq: AtomicU16::new(0),
        }
    }

    /// Generate the next unique id.
    ///
    /// Layout (high -> low): timestamp_ms (41) | node_id (10) | sequence (12)
    pub fn next_id(&self) -> i64 {
        let mut now_ms = Utc::now().timestamp_millis();
        loop {
            let last = self.last_ms.load(Ordering::SeqCst);
            if now_ms < last {
                now_ms = last;
            }

            if now_ms == last {
                let seq = self.seq.fetch_add(1, Ordering::SeqCst) & 0x0fff;
                if seq == 0 {
                    // Sequence overflow within the same millisecond.
                    while Utc::now().timestamp_millis() <= now_ms {}
                    now_ms = Utc::now().timestamp_millis();
                    continue;
                }
                return ((now_ms & 0x1ffffffffff) << 22)
                    | ((self.node_id & 0x03ff) << 12)
                    | (seq as i64);
            }

            self.last_ms.store(now_ms, Ordering::SeqCst);
            self.seq.store(0, Ordering::SeqCst);
            return ((now_ms & 0x1ffffffffff) << 22) | ((self.node_id & 0x03ff) << 12);
        }
    }
}
