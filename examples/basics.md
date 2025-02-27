# License Template Examples

Below are examples of how the custom license template would be formatted for different file types when applied by edlicense.

## Rust/Go/C++ File (.rs, .go, .cpp)

```rust
// Copyright (c) 2025 Acme Corporation
//
// This software and associated documentation files (the "Software") are the
// exclusive property of Acme Corporation. The Software is provided to you
// under the following conditions:
//
// 1. You may use, copy, and modify the Software for internal purposes only.
// 2. You may not distribute, sublicense, or make the Software available to
//    third parties without explicit written permission from Acme Corporation.
// 3. All copies or substantial portions of the Software must include this
//    copyright notice and permission statement.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED. IN NO EVENT SHALL ACME CORPORATION BE LIABLE FOR ANY CLAIM,
// DAMAGES OR OTHER LIABILITY ARISING FROM THE USE OF THE SOFTWARE.
//
// For licensing inquiries, contact: licensing@acme-example.com

fn main() {
    println!("Hello, world!");
}
```

## Python/Shell/YAML File (.py, .sh, .yaml)

```python
# Copyright (c) 2025 Acme Corporation
#
# This software and associated documentation files (the "Software") are the
# exclusive property of Acme Corporation. The Software is provided to you
# under the following conditions:
#
# 1. You may use, copy, and modify the Software for internal purposes only.
# 2. You may not distribute, sublicense, or make the Software available to
#    third parties without explicit written permission from Acme Corporation.
# 3. All copies or substantial portions of the Software must include this
#    copyright notice and permission statement.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED. IN NO EVENT SHALL ACME CORPORATION BE LIABLE FOR ANY CLAIM,
# DAMAGES OR OTHER LIABILITY ARISING FROM THE USE OF THE SOFTWARE.
#
# For licensing inquiries, contact: licensing@acme-example.com

def hello():
    print("Hello, world!")
```

## Java/Scala/Kotlin File (.java, .scala, .kt)

```java
/*
 * Copyright (c) 2025 Acme Corporation
 *
 * This software and associated documentation files (the "Software") are the
 * exclusive property of Acme Corporation. The Software is provided to you
 * under the following conditions:
 *
 * 1. You may use, copy, and modify the Software for internal purposes only.
 * 2. You may not distribute, sublicense, or make the Software available to
 *    third parties without explicit written permission from Acme Corporation.
 * 3. All copies or substantial portions of the Software must include this
 *    copyright notice and permission statement.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED. IN NO EVENT SHALL ACME CORPORATION BE LIABLE FOR ANY CLAIM,
 * DAMAGES OR OTHER LIABILITY ARISING FROM THE USE OF THE SOFTWARE.
 *
 * For licensing inquiries, contact: licensing@acme-example.com
 */

public class Hello {
    public static void main(String[] args) {
        System.out.println("Hello, world!");
    }
}
```

## JavaScript/TypeScript/CSS File (.js, .ts, .css)

```javascript
/**
 * Copyright (c) 2025 Acme Corporation
 *
 * This software and associated documentation files (the "Software") are the
 * exclusive property of Acme Corporation. The Software is provided to you
 * under the following conditions:
 *
 * 1. You may use, copy, and modify the Software for internal purposes only.
 * 2. You may not distribute, sublicense, or make the Software available to
 *    third parties without explicit written permission from Acme Corporation.
 * 3. All copies or substantial portions of the Software must include this
 *    copyright notice and permission statement.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED. IN NO EVENT SHALL ACME CORPORATION BE LIABLE FOR ANY CLAIM,
 * DAMAGES OR OTHER LIABILITY ARISING FROM THE USE OF THE SOFTWARE.
 *
 * For licensing inquiries, contact: licensing@acme-example.com
 */

function hello() {
    console.log("Hello, world!");
}
```

## HTML/XML File (.html, .xml)

```html
<!--
 Copyright (c) 2025 Acme Corporation

 This software and associated documentation files (the "Software") are the
 exclusive property of Acme Corporation. The Software is provided to you
 under the following conditions:

 1. You may use, copy, and modify the Software for internal purposes only.
 2. You may not distribute, sublicense, or make the Software available to
    third parties without explicit written permission from Acme Corporation.
 3. All copies or substantial portions of the Software must include this
    copyright notice and permission statement.

 THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 IMPLIED. IN NO EVENT SHALL ACME CORPORATION BE LIABLE FOR ANY CLAIM,
 DAMAGES OR OTHER LIABILITY ARISING FROM THE USE OF THE SOFTWARE.

 For licensing inquiries, contact: licensing@acme-example.com
-->

<!DOCTYPE html>
<html>
<head>
    <title>Hello World</title>
</head>
<body>
    <h1>Hello, world!</h1>
</body>
</html>
```

## Using with edlicense

To use this custom license template with edlicense, you would run:

```bash
edlicense --license-file custom_license_template.txt .
```

This would apply the custom license to all supported files in the current directory and its subdirectories, automatically formatting the license header according to each file type.