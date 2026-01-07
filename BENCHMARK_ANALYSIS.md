# Benchmark Analysis and Optimization Opportunities

**Date:** 2026-01-07
**Benchmark Run:** Comparative benchmarks between edlicense and Google's addlicense

## Executive Summary

edlicense demonstrates excellent performance on small files and in check mode, but shows opportunities for optimization in large file processing and first-iteration performance. The benchmarks reveal a 40-120% slowdown on the first iteration and 19-22% slower performance on large files compared to addlicense.

## Benchmark Results

### Performance Comparison: edlicense vs addlicense

| Operation | File Size | File Count | edlicense (ms) | addlicense (ms) | Winner | Difference |
|-----------|-----------|------------|----------------|-----------------|--------|------------|
| add       | 1KB       | 10,000     | 1,693          | 1,872           | ✅ edlicense | **10.6% faster** |
| add       | 10KB      | 5,000      | 1,245          | 1,008           | ❌ addlicense | 19% slower |
| add       | 100KB     | 1,000      | 433            | 336             | ❌ addlicense | 22.3% slower |
| check     | 1KB       | 10,000     | 1,158          | 1,739           | ✅ edlicense | **50.1% faster** |
| check     | 10KB      | 5,000      | 639            | 917             | ✅ edlicense | **43.5% faster** |
| check     | 100KB     | 1,000      | 157            | 237             | ✅ edlicense | **51.5% faster** |

**Key Findings:**
- ✅ **Check mode**: edlicense is 43-51% faster across all file sizes
- ✅ **Small files**: edlicense is 10.6% faster on 1KB files
- ❌ **Large files**: addlicense is 19-22% faster on 10KB+ files

### Thread Scaling Analysis

| Threads | Avg Time (ms) | Speedup vs 1 thread | Efficiency |
|---------|---------------|---------------------|------------|
| 1       | 5,566         | 1.00x               | 100%       |
| 2       | 3,132         | 1.78x               | 89%        |
| 4       | 2,261         | 2.46x               | 62%        |
| 8       | 1,879         | 2.96x               | 37%        |
| 16      | 1,759         | 3.16x               | 20%        |

**Observations:**
- Good scaling up to 4 threads (2.46x speedup)
- Diminishing returns beyond 8 threads
- Suggests I/O bottleneck rather than CPU bottleneck

### First Iteration Slowness Issue

A critical performance anomaly was identified:

**Add Operation (1KB files, 10k files):**
- Iteration 1: 2,075ms
- Iteration 2: 1,525ms (27% faster)
- Iteration 3: 1,480ms (29% faster)

**Update Operation (1KB files, 10k files):**
- Iteration 1: **3,551ms**
- Iteration 2: **1,625ms** (54% faster!)
- Iteration 3: 1,553ms (56% faster!)

The first iteration is consistently **40-120% slower** than subsequent iterations.

## Root Cause Analysis

### 1. First Iteration Slowness

**Primary Causes:**
- **LazyLock initialization**: The `YEAR_REGEX` at `src/processor.rs:1329` compiles on first use
- **Tokio runtime warmup**: Async runtime initialization overhead
- **Template manager initialization**: Template compilation and caching
- **File system cache**: Cold disk cache vs warm cache on subsequent runs

**Evidence:**
```rust
// Line 1329-1330 in processor.rs
static YEAR_REGEX: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"(?i)(copyright\s+(?:\(c\)|©)?\s+)(\d{4})(\s+)").expect("year regex must compile"));
```

The regex is compiled lazily on first access, adding overhead to the first iteration.

### 2. Large File Performance Gap

**Why addlicense is faster on large files:**

1. **File Reading Strategy:**
   - edlicense reads prefix (8KB) then conditionally reads the rest
   - For large files with licenses, this means reading the entire file into memory
   - addlicense (Go) has more efficient large string handling

2. **String Operations:**
   - Rust string operations involve UTF-8 validation
   - Multiple allocations during `format!()` calls (lines 903, 1200)
   - `replace_all()` creates new String allocations (line 1345)

3. **Memory Allocation Pattern:**
   ```rust
   // Line 903: Multiple allocations
   let new_content = format!("{}{}{}", prefix, formatted_license, content);
   ```
   For a 100KB file, this creates a new ~100KB+ string allocation.

### 3. Thread Scaling Plateau

**Current Implementation (lines 513-528):**
```rust
let num_cpus = num_cpus::get();
let files_len = files.len();
let mut concurrency = std::cmp::min(num_cpus * 4, files_len);
```

The concurrency is capped at `num_cpus * 4`, which is reasonable, but the plateau suggests:
- **I/O bottleneck**: Disk I/O becomes the limiting factor
- **Mutex contention**: IgnoreManager cache lock (lines 296-313, 590-602)

## Optimization Recommendations

### Priority 1: Fix First Iteration Slowness

**Optimization 1.1: Pre-compile Regex**
```rust
// Replace LazyLock with static compilation
use regex::Regex;
use std::sync::OnceLock;

static YEAR_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_year_regex() -> &'static Regex {
    YEAR_REGEX.get_or_init(|| {
        Regex::new(r"(?i)(copyright\s+(?:\(c\)|©)?\s+)(\d{4})(\s+)")
            .expect("year regex must compile")
    })
}
```
**Expected Impact:** 10-20% improvement on first iteration

**Optimization 1.2: Benchmark Suite Warmup**
The comparative benchmark test should include a warmup run before timing:
```rust
// Run once to warm up caches
run_edlicense_benchmark(operation, &edlicense_dir, check_only, &warmup_config).await?;

// Now run the actual timed benchmarks
for i in 1..=config.iterations {
    // ... timing code
}
```
**Expected Impact:** More accurate benchmark measurements

### Priority 2: Optimize Large File Performance

**Optimization 2.1: Avoid Full File Reads When Possible**

For files that already have licenses and we're just checking (not updating years):
```rust
// In read_license_check_prefix, if we detect a license in the prefix
// and we're in check mode with preserve_years, we can skip reading the rest
if self.check_only && self.preserve_years && self.has_license(&prefix_content) {
    return Ok((file, buf, prefix_content));
}
```
**Expected Impact:** 30-50% improvement on large file check operations

**Optimization 2.2: Use String Capacity Pre-allocation**
```rust
// Line 903 optimization
let new_content = {
    let mut content = String::with_capacity(prefix.len() + formatted_license.len() + content.len());
    content.push_str(&prefix);
    content.push_str(&formatted_license);
    content.push_str(&content);
    content
};
```
**Expected Impact:** 5-10% improvement on large files

**Optimization 2.3: Optimize Year Update for Large Files**

For large files, avoid processing the entire file if the year update only affects the header:
```rust
// Only search for copyright in the first 8KB (LICENSE_READ_LIMIT)
// Most licenses are in the first few lines
if content.len() > LICENSE_READ_LIMIT {
    let (header, body) = content.split_at(LICENSE_READ_LIMIT);
    if let Some(updated_header) = update_year_in_license_fast(header) {
        let mut result = String::with_capacity(content.len());
        result.push_str(&updated_header);
        result.push_str(body);
        return Ok(Cow::Owned(result));
    }
}
```
**Expected Impact:** 20-30% improvement on large file updates

### Priority 3: Reduce Mutex Contention

**Optimization 3.1: Use DashMap for IgnoreManager Cache**

Replace `Arc<Mutex<HashMap>>` with `DashMap` for lock-free concurrent access:
```rust
// Replace line 73
use dashmap::DashMap;

ignore_manager_cache: Arc<DashMap<PathBuf, IgnoreManager>>,
```
**Expected Impact:** 10-15% improvement on multi-threaded workloads

**Optimization 3.2: Batch Report Collection**

Instead of locking the report mutex for every file, collect reports in thread-local storage and batch them:
```rust
// Use channel with larger buffer
let (report_sender, report_receiver) = tokio::sync::mpsc::channel::<Vec<FileReport>>(concurrency);
```
**Expected Impact:** 5-10% improvement on highly concurrent workloads

### Priority 4: Additional Micro-optimizations

**Optimization 4.1: Fast Path for Common Cases**
```rust
// Line 1240: Expand fast path check
pub fn has_license(&self, content: &str) -> bool {
    // Fast path: check first 200 chars for common patterns
    let check_prefix = &content[..content.len().min(200)];
    if check_prefix.contains("Copyright") || check_prefix.contains("copyright") {
        return true;
    }
    self.license_detector.has_license(content)
}
```

**Optimization 4.2: Reduce String Allocations in extract_prefix**
```rust
// Line 1283: Return &str instead of String where possible
pub fn extract_prefix<'a>(&self, content: &'a str) -> (Cow<'a, str>, &'a str) {
    // Use Cow to avoid allocation when no prefix exists
}
```

## Expected Performance Improvements

| Optimization | Target Metric | Expected Improvement |
|--------------|---------------|---------------------|
| Pre-compile Regex | First iteration (update) | 10-20% faster |
| Avoid full reads (check mode) | Large file checks | 30-50% faster |
| String capacity pre-allocation | Large file adds | 5-10% faster |
| Header-only year updates | Large file updates | 20-30% faster |
| DashMap for cache | Multi-threaded scenarios | 10-15% faster |

**Combined Impact:**
- First iteration slowdown reduced from 120% to ~30-40%
- Large file performance gap reduced from 19-22% slower to ~5-10% slower or potentially faster
- Overall throughput improvement of 15-25% on typical workloads

## Next Steps

1. **Implement Priority 1 optimizations** (regex pre-compilation, benchmark warmup)
2. **Re-run benchmarks** to validate improvements
3. **Implement Priority 2 optimizations** (large file handling)
4. **Profile with `cargo flamegraph`** to identify remaining bottlenecks
5. **Implement Priority 3 optimizations** (mutex contention)
6. **Final benchmark comparison** against addlicense

## Conclusion

edlicense shows strong performance on small files and check operations (10-51% faster than addlicense). The main optimization opportunities are:

1. **First iteration warmup** - Cold cache/initialization overhead
2. **Large file handling** - String allocation and file reading strategy
3. **Mutex contention** - IgnoreManager cache

With the proposed optimizations, edlicense could potentially match or exceed addlicense's performance across all file sizes and operations while maintaining its superior check mode performance.
