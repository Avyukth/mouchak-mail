use super::*;

#[test]
fn test_scrubber_standard_mode_preserves_names() {
    let scrubber = Scrubber::new(ScrubMode::Standard);
    let name = "BlueMountain";
    assert_eq!(scrubber.scrub_name(name), "BlueMountain");

    let name2 = "GreenCastle";
    assert_eq!(scrubber.scrub_name(name2), "GreenCastle");
}

#[test]
fn test_scrubber_aggressive_mode_redacts_names() {
    let scrubber = Scrubber::new(ScrubMode::Aggressive);
    let name = "BlueMountain";
    assert_eq!(scrubber.scrub_name(name), "[REDACTED-NAME]");
}
