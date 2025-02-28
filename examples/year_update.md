# Automatic Year Update Feature

One of the key improvements in edlicense over the original addlicense tool is the automatic updating of copyright year references. This document demonstrates how this feature works.

## Example Scenario

Imagine you have a project with source files that were licensed in 2024:

### Original file (from 2024):

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

### When you run edlicense in 2025:

```bash
edlicense .
```

### The file is automatically updated to:

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

fn main() {
    println!("Hello, world!");
}
```

## Implementation Details

The automatic year update feature works by:

1. Detecting existing copyright year references in license headers
2. Comparing them with the current year
3. Updating the year reference if it's outdated

This feature ensures that your copyright notices stay current without manual intervention, reducing maintenance overhead and ensuring legal compliance.

## Configuration

By default, edlicense uses the current year for updates. You can override this behavior with the `--year` flag:

```bash
edlicense --year "2026" .  # Use a specific year instead of the current one
```

If you want to disable the automatic year update feature and keep the existing year references:

```bash
edlicense --preserve-years .  # This flag would need to be implemented
```
