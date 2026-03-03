# Numeric Types

Michael Stay ([director.research@f1r3fly.io](mailto:director.research@f1r3fly.io))  
2025-11-13

Rholang currently supports only one numeric type, signed 64-bit integers.  This design proposes adding a set of numeric types following the ECMAScript model, where non-default numeric literals are suffixed with a type identifier, and the numeric type must agree in all arguments to an arithmetic operator.

# Introduction

Rholang currently only supports signed 64-bit integers.  While it's possible to implement arbitrary sized integers in terms of these, it would involve writing a library to manipulate the data structures, which is worse both in performance and usability.  Ben Goertzel has asked us to support floating point arithmetic; we had chosen not to support them because if people used floating point numbers for fractional tokens, the money supply would not be conserved: `0.1 + 0.2 == 0.30000000000000004`.

# ECMAScript bigints

Until version 11, ECMAScript supported only 64-bit IEEE-754 floating point values; bitwise operators cast back and forth to 32-bit signed integers.  It adopted [bigints in June 2020](https://en.wikipedia.org/wiki/ECMAScript_version_history#11th_edition_%E2%80%93_ECMAScript_2020) using an `n` suffix to indicate that a numeric literal denoted a bigint value.  Binary arithmetic and bitwise operators (`+ - * / ** % << >> ~ & | ^`) require that all arguments agree on the numeric type.  Unsigned right shift (`>>>`) doesn't work with bigints because all bigints are signed.  Unary negation works on bigints, but unary plus doesn't because of a conflict with [asm.js](https://en.wikipedia.org/wiki/Asm.js). 

# Proposed Rholang numeric types

We propose following this pattern to extend the set of numeric types in Rholang without introducing ambiguity between numbers of different types, accidental coercion, or breaking existing code.

Platforms may not support certain numeric types defined below and should throw an error as early as possible (e.g. before deploying the contract in blockchain applications).  Dynamic casting may throw an error at runtime (causing a deployment to abort in blockchain applications).

## Default: signed 64-bit integers

So as not to change the semantics of existing code, unqualified numeric literals refer to signed 64-bit integers.

## Signed integers

Signed integers use two's complement encoding.  They come in sizes of 2^n for n ≥ 3 and are denoted `<digits>i<2^n>`, e.g. `-52i64` is the same as `-52` and `-1i256` is the 256-bit integer all of whose bits are `1`.

Division is integer division: `10i64 / 3i64 == 3i64`.  The remainder can be found using the modulo operator (`%`): `10i64 % 3i64 == 1i64`.  It can also be found using the `rem` operator (see below). We follow C99 in saying that `(a/b) * b + a%b` shall equal `a`.

## Unsigned integers

Unsigned integers come in sizes of 2^n for n ≥ 3 and are denoted `<digits>u<2^n>`, e.g. `65535u16` is the 16-bit number all of whose bits are 1\.

Division is integer division: `10u64 / 3u64 == 3u64`.  The remainder can be found using the modulo operator (`%`): `10u64 % 3u64 == 1u64`.

## Signed bigints

Signed bigints are denoted `<digits>n`.  Negation uses two's complement.  This will require some finagling: the two's complement of any finite number effectively has an infinite number of 1s to the left, so `-1n & (1n << 100n) === (1n << 100n)`.

Division is integer division: `10n / 3n == 3n`.  The remainder can be found using the modulo operator (`%`): `10n % 3n == 1n`.  It can also be found using the `rem` operator (see below). We follow C99 in saying that `(a/b) * b + a%b` shall equal `a`.

## Signed bigrats

Bigrats are ratios of bigints.  They're denoted `<digits>r`.  Division is rational division: `10r / 3r == 3r + 1r / 3r`.  The modulus operator always gives 0 on bigrats, since `(a/b) * b == a` exactly.  Negation uses two's complement, and bitwise operators can act on numbers less than 1: `1r/3r & 3r/4r == 1r/4r` since 1/3 \== 0.010101... and 3/4 \== 0.11000...

## IEEE 754 floating point

Floating point numbers come in four sizes: single, double, quadruple, and octuple precision. They're denoted as in C99 but they append the suffixes `f32`, `f64`, `f128`, and `f256`, respectively.  For example, `-1.234e5f32 == -123400f32`. The modulus operator and bitwise operators are not defined on floating point numbers and should cause an error as early as possible.

## Fixed point

Fixed point numbers are basically bigints where the decimal point has been shifted.  They're denoted `<digits>p<digits>` (integral), `<digits>.<optional digits>p<digits>` (leading digits), or `.<digits>p<digits>` (no leading digits).

Division is shifted integer division: `10p1 / 3p1 == 3.3p1`.  We follow C99 in saying that `(a/b) * b + a%b` shall equal `a`, so `10p1 % 3p1 == 0.1p1`.

Bitwise operators should satisfy `x op y = bigrat(x) op bigrat(y)`.

# Casting

Conversion between numeric types must be explicit; we propose various builtins for that purpose:

- **int(arg, m)** casts **arg** to a signed **m**\-bit integer, where **m** is a signed 64-bit integer.  When m \= 2^n for n ≥ 3, it rounds **arg** towards \-∞, so `int(-3.5f32, 8) == -4i8`.  Otherwise, it throws an error.  
- **uint(arg, m)** casts **arg** to a signed **m**\-bit integer, where **m** is a signed 64-bit integer.  When m \= 2^n for n ≥ 3, it rounds **arg** towards \-∞, so `uint(3.5f32, 8) == 3u8`.  Negative numbers round to 0, so `uint(-3.5f32, 8) == 0u8`.  If **m** is otherwise, it throws an error.  
- **bigint(arg)** casts **arg** to a signed bigint.  It rounds **arg** towards \-∞, so `bigint(-3.5f32) == -4n`.  
- **bigrat(arg)** casts **arg** to a signed bigrat.  Casting from non-floating-point numbers is exact.  When casting from a floating point number, it takes the least value in the range denoted by the floating point value.  
- **float(arg, m)** casts **arg** to an IEEE 754 floating point number, where **m** is a signed 64-bit integer. When **m** is one of 32, 64, 80, 128, or 256, it returns the nearest floating point value of that width (which may be `±Inf`).  If **m** is otherwise, it throws an error.  
- **fixed(arg, m)** casts **arg** to a fixed point number with **m** digits after the decimal point, where **m** is a signed 64-bit integer.  It rounds towards \-∞, so `fixed(3.49p2, 1) == 3.4p1` and `fixed(-3.49p2, 1) == -3.5p1`.  When casting from a floating point value, it takes the least value in the range denoted by the floating point value.

When casting from a larger integer type to a smaller one, the cast uses modular arithmetic: `uint(257u16, 8) == 1u8`.  When casting from a signed int to an unsigned int, the two's complement form is preserved: `uint(-1n, 8) == 255u8`.  Casting from a noninteger type of a larger size to a smaller float type may result in `±Inf`: `float(1e50f64, 32) == Inf`.  Casting `Inf` or `NaN` to any non-float type results in an overflow error.  Casting from a noninteger type to an integer type results in an overflow error if the floor of the noninteger is not in range.

# Alternatives

* **Directed acyclic graph of coercions**. C/C++/Java/Scheme/etc. have implicit coercions of smaller numeric types to larger ones.  
* **Multiple dispatch.** Rather than have coercion as part of the language, Julia defines binary arithmetic functions at each pair of types and implements the coercion explicitly in those functions.  We could conceivably do this with patterns in a purely functional expression sublanguage.  This would also enable custom user-defined numeric types.  This is consistent with the design above.

# Open Question: Units

There have been some [dramatic failures](https://www.simscale.com/blog/nasa-mars-climate-orbiter-metric/) due to using the wrong units. Should we adopt syntax for specifying units [like F\# does](https://learn.microsoft.com/en-us/dotnet/fsharp/language-reference/units-of-measure) as well as the numeric type?  We would only be allowed to add, subtract, multiply, and divide unitful numbers; addition and subtraction would require the arguments to have the same units.  Exponentiation and bitwise operations would only work on unitless numbers.

Open Question: Runtime conversions

In a case such as: x / 30u64 how do we evaluate `x` \- since the language is untyped we can upcast it to the “bigger” type of two which seems to be the approach in dynamic languages, but is it what we want?

`for(@x <- chan) { y!(x / 4) }`

Instead of 10.23f64, f64\`10.23\`  
foo\`myformat\`