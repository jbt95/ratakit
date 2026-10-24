//! Normalized async event stream over crossterm.

use std::io;

use futures_util::Stream;
use futures_util::stream::StreamExt;
use ratakit_core::Event;

use crate::crossterm::from_crossterm_event;

/// Async stream of normalized [`Event`]s from the terminal.
///
/// Wraps crossterm's [`EventStream`](crossterm::event::EventStream).
pub fn event_stream() -> impl Stream<Item = io::Result<Event>> {
    crossterm::event::EventStream::new().map(|res| res.map(from_crossterm_event))
}
