# ubid

Fixed-width random identifiers with a binary representation and lowercase Crockford base32 text
encoding.

UBIDs are useful when you want compact, opaque IDs that can be generated without coordination. The
crate provides a generic `Ubid<N>` type, where `N` is the number of random bytes, plus common
aliases:

| Alias | Bytes | Bits | Base32 characters | Example | Typical use |
| --- | ---: | ---: | ---: | --- | --- |
| `Ubid40` | 5 | 40 | 8 | `a8mhvtc9` | Small spaces where collisions can be detected and handled |
| `Ubid80` | 10 | 80 | 16 | `q08wg7jan2mnp0t0` | Larger spaces where collisions can be detected and handled |
| `Ubid120` | 15 | 120 | 24 | `v18008kg68djyx6hfpvpmpe4` | High-cardinality IDs without coordination |
| `Ubid160` | 20 | 160 | 32 | `n2zeseq2ccxtbdq60j2bd03qrbwxw2rv` | Conservative choice with the largest collision margin |

## Motivation

UUIDv4 is a good default identifier, but it is not a perfect fit when the goal is a compact
Crockford base32 string. It stores 122 bits of random entropy in a 16-byte layout because the
version and variant fields consume 6 bits. Encoding those 16 bytes as base32 also needs 26
characters, with the final character only partially filled.

UBID uses widths that are both byte-aligned and base32-aligned. Every 5 bytes become exactly 8
base32 characters.

## Example

```rust
use ubid::Ubid120;

let id = Ubid120::generate();
let encoded = id.to_string();
let decoded: Ubid120 = encoded.parse().unwrap();

assert_eq!(id, decoded);
```

## Command Line

The `ubidgen` binary generates one ID for each requested width:

```console
$ cargo install ubid
```

```console
$ ubidgen 80 120
q08wg7jan2mnp0t0
v18008kg68djyx6hfpvpmpe4
```

## Choosing A Width

Because UBIDs are uniformly random, collision risk follows the birthday bound. After generating `n`
IDs in a `b`-bit space, the collision probability is approximately `n^2 / 2^(b + 1)` while that
probability is small.

Use the table above as a starting point, then choose the smallest width that gives enough collision
margin for the number of IDs you expect to generate.

`Ubid120` is the compact choice for high-cardinality systems: it is 24 characters in base32 and has
roughly UUIDv4-scale collision resistance. `Ubid160` is useful when the extra 8 characters are
acceptable and the ID space is shared across very large volumes, many independent systems, or long
retention windows where a wider margin is preferred.

## Features

- `bytes`: enables conversion from `Ubid<N>` into `bytes::Bytes`.
- `proptest`: implements `proptest::arbitrary::Arbitrary` for `Ubid<N>`.

## License

MIT
