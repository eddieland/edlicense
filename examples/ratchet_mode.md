# Ratchet Mode Feature

edlicense's "ratchet mode" allows you to add license headers only to files that don't already have them, preserving existing license headers. This is particularly useful when:

1. You want to ensure all files have licenses without modifying existing ones
2. You're working with a mixed codebase where some files already have proper licensing
3. You want to avoid unnecessary git changes to files with correct licenses

## Example Scenario

Consider a project with two files - one with a license header and one without:

### File with existing license (main.rs):

```rust
// Copyright (c) 2024 Acme Corporation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

fn main() {
    println!("Hello, world!");
}
```

### File without license (utils.rs):

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn subtract(a: i32, b: i32) -> i32 {
    a - b
}
```

## Running edlicense with Ratchet Mode

When you run edlicense with the ratchet mode flag:

```bash
edlicense --ratchet .
```

### Results:

1. `main.rs` remains unchanged, preserving its existing license header
2. `utils.rs` gets a new license header added:

```rust
// Copyright (c) 2025 Acme Corporation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn subtract(a: i32, b: i32) -> i32 {
    a - b
}
```

## Implementation Details

Ratchet mode works by:

1. Scanning each file to detect if it already has a license header
2. Skipping files that already have license headers
3. Adding license headers only to files without them

## Use Cases

Ratchet mode is particularly useful in these scenarios:

- When integrating edlicense into an existing project with mixed licensing
- During continuous integration to ensure all new files have proper licensing
- When you want to avoid unnecessary git changes to files that already have correct licenses

## Command Line Usage

```bash
edlicense --ratchet origin/main --license-file custom_license_template.txt .
```
