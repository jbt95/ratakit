//! Versioned JSON Lines recording for agent events and permission prompts.
//!
//! Recording is opt-in via the `recording` feature. The writer never chooses a file path or
//! stores data on its own; applications own those decisions and can redact values before writing.

use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::state::{AgentEvent, PermissionRequest};

/// Current version of the ratakit recording line format.
pub const RECORDING_VERSION: u16 = 1;

/// One replayable item from a recording.
#[derive(Debug, Clone, PartialEq)]
pub enum RecordingEntry {
    Event(AgentEvent),
    Permission(PermissionRequest),
    AwaitSubmit,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum VersionedLine {
    Header { version: u16 },
    Event { event: AgentEvent },
    Permission { permission: PermissionRequest },
    AwaitSubmit,
}

/// Writes versioned recording entries as JSON Lines.
pub struct RecordingWriter<W> {
    writer: W,
}

impl<W: Write> RecordingWriter<W> {
    /// Start a recording and write its format header.
    ///
    /// # Errors
    ///
    /// Returns an error if writing or flushing the format header fails.
    pub fn new(mut writer: W) -> io::Result<Self> {
        write_line(
            &mut writer,
            &VersionedLine::Header {
                version: RECORDING_VERSION,
            },
        )?;
        Ok(Self { writer })
    }

    /// Record an agent event without modifying it.
    ///
    /// # Errors
    ///
    /// Returns an error if writing or flushing the event fails.
    pub fn event(&mut self, event: &AgentEvent) -> io::Result<()> {
        self.write(&VersionedLine::Event {
            event: event.clone(),
        })
    }

    /// Apply caller-owned redaction to a copy before recording an event.
    ///
    /// # Errors
    ///
    /// Returns an error if writing or flushing the redacted event fails.
    pub fn event_with_redaction(
        &mut self,
        event: &AgentEvent,
        redact: impl FnOnce(&mut AgentEvent),
    ) -> io::Result<()> {
        let mut redacted = event.clone();
        redact(&mut redacted);
        self.event(&redacted)
    }

    /// Record a permission request without modifying it.
    ///
    /// # Errors
    ///
    /// Returns an error if writing or flushing the permission request fails.
    pub fn permission(&mut self, permission: &PermissionRequest) -> io::Result<()> {
        self.write(&VersionedLine::Permission {
            permission: permission.clone(),
        })
    }

    /// Apply caller-owned redaction to a copy before recording a permission request.
    ///
    /// # Errors
    ///
    /// Returns an error if writing or flushing the redacted permission request fails.
    pub fn permission_with_redaction(
        &mut self,
        permission: &PermissionRequest,
        redact: impl FnOnce(&mut PermissionRequest),
    ) -> io::Result<()> {
        let mut redacted = permission.clone();
        redact(&mut redacted);
        self.permission(&redacted)
    }

    /// Mark that replay should wait for the application's next submitted prompt.
    ///
    /// # Errors
    ///
    /// Returns an error if writing or flushing the marker fails.
    pub fn await_submit(&mut self) -> io::Result<()> {
        self.write(&VersionedLine::AwaitSubmit)
    }

    /// Flush pending data to the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns an error if flushing the wrapped writer fails.
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    /// Flush the recording and return the wrapped writer.
    ///
    /// # Errors
    ///
    /// Returns an error if flushing the wrapped writer fails.
    pub fn into_inner(mut self) -> io::Result<W> {
        self.flush()?;
        Ok(self.writer)
    }

    fn write(&mut self, line: &VersionedLine) -> io::Result<()> {
        write_line(&mut self.writer, line)
    }
}

fn write_line(writer: &mut impl Write, line: &VersionedLine) -> io::Result<()> {
    serde_json::to_writer(&mut *writer, line).map_err(io::Error::other)?;
    writer.write_all(b"\n")?;
    writer.flush()
}

/// Reads versioned recordings and legacy example recordings.
pub struct RecordingReader<R> {
    reader: R,
    line_number: usize,
}

impl<R: BufRead> RecordingReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            line_number: 0,
        }
    }

    /// Read the next entry, skipping blank lines, comments, and the version header.
    ///
    /// # Errors
    ///
    /// Returns an I/O error or an `InvalidData` error for malformed or unsupported recording
    /// lines.
    pub fn next_entry(&mut self) -> io::Result<Option<RecordingEntry>> {
        let mut line = String::new();
        loop {
            line.clear();
            if self.reader.read_line(&mut line)? == 0 {
                return Ok(None);
            }
            self.line_number += 1;
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let entry = parse_line(line).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("recording line {}: {error}", self.line_number),
                )
            })?;
            if let Some(entry) = entry {
                return Ok(Some(entry));
            }
        }
    }
}

fn parse_line(line: &str) -> Result<Option<RecordingEntry>, String> {
    let value: Value = serde_json::from_str(line).map_err(|error| error.to_string())?;
    if value.get("kind").is_some() {
        let line: VersionedLine =
            serde_json::from_value(value).map_err(|error| error.to_string())?;
        return match line {
            VersionedLine::Header { version } if version == RECORDING_VERSION => Ok(None),
            VersionedLine::Header { version } => {
                Err(format!("unsupported recording version {version}"))
            }
            VersionedLine::Event { event } => Ok(Some(RecordingEntry::Event(event))),
            VersionedLine::Permission { permission } => {
                Ok(Some(RecordingEntry::Permission(permission)))
            }
            VersionedLine::AwaitSubmit => Ok(Some(RecordingEntry::AwaitSubmit)),
        };
    }

    // Read recordings created by the original ACP examples before the shared API existed.
    if value
        .get("await_submit")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(Some(RecordingEntry::AwaitSubmit));
    }
    if let Some(permission) = value.get("permission") {
        let permission = serde_json::from_value(permission.clone()).map_err(|e| e.to_string())?;
        return Ok(Some(RecordingEntry::Permission(permission)));
    }
    let event = serde_json::from_value(value).map_err(|error| error.to_string())?;
    Ok(Some(RecordingEntry::Event(event)))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::state::{AgentEvent, Level};

    use super::{RECORDING_VERSION, RecordingEntry, RecordingReader, RecordingWriter};

    #[test]
    fn versioned_recording_round_trips_and_redacts_a_copy() {
        let event = AgentEvent::Notice {
            level: Level::Info,
            text: "private path /home/user/project".into(),
        };
        let mut writer = RecordingWriter::new(Vec::new()).expect("header");
        writer
            .event_with_redaction(&event, |event| {
                if let AgentEvent::Notice { text, .. } = event {
                    *text = "[redacted]".into();
                }
            })
            .expect("event");
        writer.await_submit().expect("turn marker");
        let bytes = writer.into_inner().expect("flush");

        let contents = String::from_utf8(bytes).expect("utf-8 jsonl");
        assert!(contents.contains(&format!("\"version\":{RECORDING_VERSION}")));
        assert!(!contents.contains("/home/user/project"));
        let mut reader = RecordingReader::new(Cursor::new(contents.as_bytes()));
        assert!(matches!(
            reader.next_entry().expect("read event"),
            Some(RecordingEntry::Event(AgentEvent::Notice { text, .. })) if text == "[redacted]"
        ));
        assert_eq!(
            reader.next_entry().expect("turn marker"),
            Some(RecordingEntry::AwaitSubmit)
        );
        assert_eq!(reader.next_entry().expect("end of recording"), None);
    }

    #[test]
    fn reader_accepts_legacy_lines_and_rejects_future_versions() {
        let legacy = concat!(
            "{\"await_submit\":true}\n",
            "{\"Notice\":{\"level\":\"Info\",\"text\":\"hello\"}}\n"
        );
        let mut reader = RecordingReader::new(Cursor::new(legacy));
        assert_eq!(
            reader.next_entry().expect("marker"),
            Some(RecordingEntry::AwaitSubmit)
        );
        assert!(matches!(
            reader.next_entry().expect("event"),
            Some(RecordingEntry::Event(_))
        ));

        let future = "{\"kind\":\"header\",\"version\":999}\n";
        let error = RecordingReader::new(Cursor::new(future))
            .next_entry()
            .expect_err("unsupported version");
        assert!(
            error
                .to_string()
                .contains("unsupported recording version 999")
        );
    }
}
