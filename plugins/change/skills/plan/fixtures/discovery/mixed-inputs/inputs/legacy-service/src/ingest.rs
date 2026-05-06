// Legacy traffic-ingest handlers. Two entry points.

pub fn submit(payload: &[u8]) -> Result<(), IngestError> {
    // Persist a single ingest payload to the primary Kafka topic.
    // POST /ingest — called by upstream producers.
    kafka::publish("ingest.primary", payload)
}

pub fn replay(batch_id: &str) -> Result<usize, IngestError> {
    // Re-publish a previously captured batch from Redis.
    // POST /ingest/replay — called during incident recovery.
    let items = redis::read_batch(batch_id)?;
    for item in &items {
        kafka::publish("ingest.primary", item)?;
    }
    Ok(items.len())
}

pub enum IngestError {
    Kafka,
    Redis,
}

mod kafka {
    pub fn publish(_topic: &str, _payload: &[u8]) -> Result<(), super::IngestError> {
        Ok(())
    }
}

mod redis {
    pub fn read_batch(_batch_id: &str) -> Result<Vec<Vec<u8>>, super::IngestError> {
        Ok(vec![])
    }
}
