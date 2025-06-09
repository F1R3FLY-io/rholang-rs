---
name: Bug Report
about: Create a report to help us improve Rholang-RS
title: '[BUG] '
labels: 'bug'
assignees: ''
---

## Bug Description

**Describe the bug**
A clear and concise description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. Run command '...'
2. Input Rholang code '...'
3. See error

**Expected behavior**
A clear and concise description of what you expected to happen.

**Actual behavior**
What actually happened instead.

## Environment

**System Information:**
- OS: [e.g. macOS 14.0, Ubuntu 22.04, Windows 11]
- Architecture: [e.g. x86_64, ARM64]
- Rust version: [output of `rustc --version`]
- Rholang-RS version: [output of `cargo run -p shell -- --version`]

**Build Configuration:**
- Debug or Release: [e.g. Debug]
- Features enabled: [e.g. with-file-history]
- Cargo.toml modifications: [if any]

## Code Examples

**Minimal Rholang code to reproduce:**
```rholang
// Paste the minimal Rholang code that triggers the bug
new stdout(`rho:io:stdout`) in {
  stdout!("Hello, World!")
}
```

**Shell commands:**
```bash
# Commands used to trigger the bug
cargo run -p shell
```

## Error Output

**Console output:**
```
Paste the complete error output here, including stack traces
```

**Log files:**
If applicable, attach relevant log files.

## Analysis

**Suspected cause:**
If you have any ideas about what might be causing this bug.

**Workaround:**
If you found a way to work around this issue.

**Impact Assessment:**
- [ ] Blocks development
- [ ] Causes incorrect results
- [ ] Performance degradation
- [ ] Security vulnerability
- [ ] Poor user experience

## Additional Context

**Screenshots:**
If applicable, add screenshots to help explain your problem.

**Related Issues:**
Link to any related issues or discussions.

**Regression Information:**
- [ ] This worked in a previous version
- [ ] This is a new issue
- [ ] Not sure

If this is a regression, please specify the last working version.