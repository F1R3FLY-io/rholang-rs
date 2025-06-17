# Rholang JNI Bridge

This crate provides JNI bindings for the Rholang parser, allowing Java applications to parse Rholang code using the Rust implementation.

## Overview

The Rholang JNI Bridge serves as a connector between Java applications (such as the JetBrains plugin) and the Rust-based Rholang parser. It exposes functions that can be called from Java using the Java Native Interface (JNI).

## Features

- Check if Rholang code is valid
- Parse Rholang code and return a structured result
- Support for both standard JNI and j4rs interfaces

## Usage

### From Java

```java
import org.rholang.lang.parser.RholangParserJNI;

public class Example {
    public static void main(String[] args) {
        RholangParserJNI parser = new RholangParserJNI();
        
        // Check if code is valid
        boolean isValid = parser.isValid("new channel in { @\"stdout\"!(\"Hello, world!\") }");
        
        // Parse code and get result as JSON
        String result = parser.parse("new channel in { @\"stdout\"!(\"Hello, world!\") }");
        
        System.out.println("Is valid: " + isValid);
        System.out.println("Parse result: " + result);
    }
}
```

### Building

To build the JNI bridge:

```bash
cargo build --release -p rholang-jni-bridge
```

Or use the Makefile target:

```bash
make build-rholang-jni-bridge
```

## Implementation Details

The crate provides two sets of JNI functions:

1. Standard JNI functions:
   - `Java_org_rholang_lang_parser_RholangParserJNI_isValid`
   - `Java_org_rholang_lang_parser_RholangParserJNI_parse`

2. Native JNI functions:
   - `Java_org_rholang_lang_parser_RholangParserJNI_isValidNative`
   - `Java_org_rholang_lang_parser_RholangParserJNI_parseNative`

Both sets provide the same functionality but with different function names to support different Java implementations.