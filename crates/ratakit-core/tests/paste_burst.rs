use std::time::Instant;

use ratakit_core::PasteBurst;

#[test]
fn rapid_chars_activate_burst() {
    let mut burst = PasteBurst::new();
    let t0 = Instant::now();
    burst.on_char(t0);
    burst.on_char(t0);
    burst.on_char(t0);
    burst.on_char(t0);
    assert!(burst.is_active());
    assert!(burst.enter_inserts_newline(t0));
}

#[test]
fn paste_activates_burst() {
    let mut burst = PasteBurst::new();
    let t0 = Instant::now();
    burst.on_paste(t0);
    assert!(burst.enter_inserts_newline(t0));
}
