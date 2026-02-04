# Plan: Strict Content-Based License Detector

## Summary

Add a `--strict` flag that uses `ContentBasedLicenseDetector` instead of `SimpleLicenseDetector`. Users who need precise license verification can opt-in, accepting the latency cost.

## Background

### Current Detection

edlicense has two license detection implementations in `src/license_detection.rs`:

| Detector | Method | Trade-off |
|----------|--------|-----------|
| `SimpleLicenseDetector` (default) | Searches for "copyright" keyword in first 1000 bytes | Fast, but can have false positives/negatives |
| `ContentBasedLicenseDetector` | Normalizes and compares actual license text | Slower, but precise |

The `ContentBasedLicenseDetector` exists but isn't exposed to CLI users.

### Use Cases for Strict Mode

1. **Compliance audits**: Verify exact license text is present, not just "copyright" keyword
2. **CI gates**: Strict verification before deployment
3. **False positive mitigation**: Files mentioning "copyright" in code/comments but lacking actual headers

## Proposed Design

### CLI

```
--strict    Enable strict license detection (more accurate, slower)
```

### Config File

```toml
[detection]
strict = true
```

## Implementation

### Changes Required

| File | Change |
|------|--------|
| `src/cli/check.rs` | Add `--strict` flag |
| `src/config.rs` | Add `[detection]` section with `strict` bool |
| `src/cli/check.rs` | Select detector based on strict flag |

### CLI Argument

```rust
/// Use strict content-based license detection (more accurate, slower)
#[arg(long)]
pub strict: bool,
```

### Detector Selection

In `run_check()`, select the detector based on the flag:

```rust
let detector: Box<dyn LicenseDetector> = if args.strict || config.detection.strict {
    Box::new(ContentBasedLicenseDetector::new(&license_text, None))
} else {
    Box::new(SimpleLicenseDetector::new())
};
```

### Config Parsing

```rust
#[derive(Debug, Default, Deserialize)]
pub struct DetectionConfig {
    #[serde(default)]
    pub strict: bool,
}
```

## Usage Examples

```bash
# Strict check for compliance audit
edlicense --license-file LICENSE.txt --strict .

# Strict check with report
edlicense --license-file LICENSE.txt --strict --report-json compliance.json .

# Strict mode in CI
edlicense --license-file LICENSE.txt --strict --ratchet=origin/main .
```

Or via config:

```toml
# .edlicense.toml
[detection]
strict = true
```

## Testing

1. Test that `--strict` flag selects `ContentBasedLicenseDetector`
2. Test that config `strict = true` selects `ContentBasedLicenseDetector`
3. Test that CLI flag overrides config
4. Test strict mode catches cases simple mode misses (file with "copyright" in code but no header)

## Future Considerations (Out of Scope)

- Blacklist for excluding files from strict checking
- Per-pattern or per-extension strict mode
- Performance benchmarking and optimization
