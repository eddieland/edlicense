use anyhow::Result;

// Import the logging module
use edlicense::logging::{is_verbose, set_verbose};

#[test]
fn test_verbose_flag() -> Result<()> {
    // Test default state (should be false)
    assert!(!is_verbose());

    // Set verbose to true
    set_verbose(true);
    assert!(is_verbose());

    // Set verbose to false
    set_verbose(false);
    assert!(!is_verbose());

    Ok(())
}
