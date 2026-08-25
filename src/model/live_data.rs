//! Tolerant serde DTOs mirroring the Live Client Data API `allgamedata`
//! payload. Option-everywhere: absent, null, and omitted fields are all
//! represented as `None` — never defaulted to fabricated values.
