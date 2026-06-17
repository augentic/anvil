use std::time::Duration;

use super::*;

#[test]
fn converts_unix_epoch() {
    let ts = system_time_to_utc(SystemTime::UNIX_EPOCH).expect("epoch is representable");
    assert_eq!(ts.as_second(), 0);
}

#[test]
fn converts_known_offset() {
    let when = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let ts = system_time_to_utc(when).expect("offset is representable");
    assert_eq!(ts.as_second(), 1_700_000_000);
}
