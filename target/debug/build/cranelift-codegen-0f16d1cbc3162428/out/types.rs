/// An integer type with 8 bits.
/// WARNING: arithmetic on 8bit integers is incomplete
pub const I8: Type = Type(0x74);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// An integer type with 16 bits.
/// WARNING: arithmetic on 16bit integers is incomplete
pub const I16: Type = Type(0x75);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// An integer type with 32 bits.
pub const I32: Type = Type(0x76);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// An integer type with 64 bits.
pub const I64: Type = Type(0x77);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// An integer type with 128 bits.
pub const I128: Type = Type(0x78);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A 16-bit floating point type represented in the IEEE 754-2008
/// *binary16* interchange format. This corresponds to the :c:type:`_Float16`
/// type in most C implementations.
/// WARNING: f16 support is a work-in-progress and is incomplete
pub const F16: Type = Type(0x79);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A 32-bit floating point type represented in the IEEE 754-2008
/// *binary32* interchange format. This corresponds to the :c:type:`float`
/// type in most C implementations.
pub const F32: Type = Type(0x7a);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A 64-bit floating point type represented in the IEEE 754-2008
/// *binary64* interchange format. This corresponds to the :c:type:`double`
/// type in most C implementations.
pub const F64: Type = Type(0x7b);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A 128-bit floating point type represented in the IEEE 754-2008
/// *binary128* interchange format. This corresponds to the :c:type:`_Float128`
/// type in most C implementations.
/// WARNING: f128 support is a work-in-progress and is incomplete
pub const F128: Type = Type(0x7c);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 2 lanes containing a `i8` each.
pub const I8X2: Type = Type(0x84);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 2 lanes containing `i8` bits each.
pub const I8X2XN: Type = Type(0x104);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 4 lanes containing a `i8` each.
pub const I8X4: Type = Type(0x94);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 2 lanes containing a `i16` each.
pub const I16X2: Type = Type(0x85);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 2 lanes containing a `f16` each.
pub const F16X2: Type = Type(0x89);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 4 lanes containing `i8` bits each.
pub const I8X4XN: Type = Type(0x114);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 2 lanes containing `i16` bits each.
pub const I16X2XN: Type = Type(0x105);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 2 lanes containing `f16` bits each.
pub const F16X2XN: Type = Type(0x109);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 8 lanes containing a `i8` each.
pub const I8X8: Type = Type(0xa4);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 4 lanes containing a `i16` each.
pub const I16X4: Type = Type(0x95);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 2 lanes containing a `i32` each.
pub const I32X2: Type = Type(0x86);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 4 lanes containing a `f16` each.
pub const F16X4: Type = Type(0x99);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 2 lanes containing a `f32` each.
pub const F32X2: Type = Type(0x8a);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 8 lanes containing `i8` bits each.
pub const I8X8XN: Type = Type(0x124);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 4 lanes containing `i16` bits each.
pub const I16X4XN: Type = Type(0x115);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 2 lanes containing `i32` bits each.
pub const I32X2XN: Type = Type(0x106);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 4 lanes containing `f16` bits each.
pub const F16X4XN: Type = Type(0x119);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 2 lanes containing `f32` bits each.
pub const F32X2XN: Type = Type(0x10a);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 16 lanes containing a `i8` each.
pub const I8X16: Type = Type(0xb4);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 8 lanes containing a `i16` each.
pub const I16X8: Type = Type(0xa5);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 4 lanes containing a `i32` each.
pub const I32X4: Type = Type(0x96);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 2 lanes containing a `i64` each.
pub const I64X2: Type = Type(0x87);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 8 lanes containing a `f16` each.
pub const F16X8: Type = Type(0xa9);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 4 lanes containing a `f32` each.
pub const F32X4: Type = Type(0x9a);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 2 lanes containing a `f64` each.
pub const F64X2: Type = Type(0x8b);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 16 lanes containing `i8` bits each.
pub const I8X16XN: Type = Type(0x134);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 8 lanes containing `i16` bits each.
pub const I16X8XN: Type = Type(0x125);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 4 lanes containing `i32` bits each.
pub const I32X4XN: Type = Type(0x116);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 2 lanes containing `i64` bits each.
pub const I64X2XN: Type = Type(0x107);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 8 lanes containing `f16` bits each.
pub const F16X8XN: Type = Type(0x129);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 4 lanes containing `f32` bits each.
pub const F32X4XN: Type = Type(0x11a);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 2 lanes containing `f64` bits each.
pub const F64X2XN: Type = Type(0x10b);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 32 lanes containing a `i8` each.
pub const I8X32: Type = Type(0xc4);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 16 lanes containing a `i16` each.
pub const I16X16: Type = Type(0xb5);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 8 lanes containing a `i32` each.
pub const I32X8: Type = Type(0xa6);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 4 lanes containing a `i64` each.
pub const I64X4: Type = Type(0x97);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 2 lanes containing a `i128` each.
pub const I128X2: Type = Type(0x88);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 16 lanes containing a `f16` each.
pub const F16X16: Type = Type(0xb9);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 8 lanes containing a `f32` each.
pub const F32X8: Type = Type(0xaa);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 4 lanes containing a `f64` each.
pub const F64X4: Type = Type(0x9b);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 2 lanes containing a `f128` each.
pub const F128X2: Type = Type(0x8c);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 32 lanes containing `i8` bits each.
pub const I8X32XN: Type = Type(0x144);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 16 lanes containing `i16` bits each.
pub const I16X16XN: Type = Type(0x135);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 8 lanes containing `i32` bits each.
pub const I32X8XN: Type = Type(0x126);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 4 lanes containing `i64` bits each.
pub const I64X4XN: Type = Type(0x117);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 2 lanes containing `i128` bits each.
pub const I128X2XN: Type = Type(0x108);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 16 lanes containing `f16` bits each.
pub const F16X16XN: Type = Type(0x139);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 8 lanes containing `f32` bits each.
pub const F32X8XN: Type = Type(0x12a);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 4 lanes containing `f64` bits each.
pub const F64X4XN: Type = Type(0x11b);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 2 lanes containing `f128` bits each.
pub const F128X2XN: Type = Type(0x10c);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 64 lanes containing a `i8` each.
pub const I8X64: Type = Type(0xd4);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 32 lanes containing a `i16` each.
pub const I16X32: Type = Type(0xc5);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 16 lanes containing a `i32` each.
pub const I32X16: Type = Type(0xb6);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 8 lanes containing a `i64` each.
pub const I64X8: Type = Type(0xa7);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 4 lanes containing a `i128` each.
pub const I128X4: Type = Type(0x98);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 32 lanes containing a `f16` each.
pub const F16X32: Type = Type(0xc9);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 16 lanes containing a `f32` each.
pub const F32X16: Type = Type(0xba);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 8 lanes containing a `f64` each.
pub const F64X8: Type = Type(0xab);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A SIMD vector with 4 lanes containing a `f128` each.
pub const F128X4: Type = Type(0x9c);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 64 lanes containing `i8` bits each.
pub const I8X64XN: Type = Type(0x154);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 32 lanes containing `i16` bits each.
pub const I16X32XN: Type = Type(0x145);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 16 lanes containing `i32` bits each.
pub const I32X16XN: Type = Type(0x136);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 8 lanes containing `i64` bits each.
pub const I64X8XN: Type = Type(0x127);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 4 lanes containing `i128` bits each.
pub const I128X4XN: Type = Type(0x118);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 32 lanes containing `f16` bits each.
pub const F16X32XN: Type = Type(0x149);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 16 lanes containing `f32` bits each.
pub const F32X16XN: Type = Type(0x13a);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 8 lanes containing `f64` bits each.
pub const F64X8XN: Type = Type(0x12b);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
/// A dynamically-scaled SIMD vector with a minimum of 4 lanes containing `f128` bits each.
pub const F128X4XN: Type = Type(0x11c);
 // /Users/andrewweston/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cranelift-codegen-meta-0.129.1/src/gen_types.rs:19
