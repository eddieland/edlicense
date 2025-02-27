use anyhow::Result;
use std::sync::atomic::Ordering;

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

// Note: The macro tests are commented out because they would require
// capturing stderr/stdout which is difficult in a test environment.
// In a real-world scenario, these would be tested with a more sophisticated
// test harness or integration tests.

/*
#[test]
fn test_verbose_log_macro() -> Result<()> {
    // Set up test environment

    // Test with verbose mode off
    set_verbose(false);
    // Use the macro through the crate

    // Test with verbose mode on
    set_verbose(true);
    // Use the macro through the crate

    // Reset verbose mode
    set_verbose(false);

    Ok(())
}

#[test]
fn test_info_log_macro() -> Result<()> {
    // Set up test environment

    // Test info_log (should always log regardless of verbose setting)
    set_verbose(false);
    // Use the macro through the crate

    // Reset verbose mode
    set_verbose(false);

    Ok(())
}
*/
