// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(rustc_private)]

extern crate rustc_llvm as llvm;
extern crate flate;

use llvm::archive_ro::ArchiveRO;
use std::env;
use std::fs::File;
use std::path::Path;
use std::io::Write;

// RLIB LLVM-BYTECODE OBJECT LAYOUT
// Version 1
// Bytes    Data
// 0..10    "RUST_OBJECT" encoded in ASCII
// 11..14   format version as little-endian u32
// 15..22   size in bytes of deflate compressed LLVM bitcode as
//          little-endian u64
// 23..     compressed LLVM bitcode

// This is the "magic number" expected at the beginning of a LLVM bytecode
// object in an rlib.
pub const RLIB_BYTECODE_OBJECT_MAGIC: &'static [u8] = b"RUST_OBJECT";

// The version number this compiler will write to bytecode objects in rlibs
pub const RLIB_BYTECODE_OBJECT_VERSION: u32 = 1;

// The offset in bytes the bytecode object format version number can be found at
pub const RLIB_BYTECODE_OBJECT_VERSION_OFFSET: usize = 11;

// The offset in bytes the size of the compressed bytecode can be found at in
// format version 1
pub const RLIB_BYTECODE_OBJECT_V1_DATASIZE_OFFSET: usize =
    RLIB_BYTECODE_OBJECT_VERSION_OFFSET + 4;

// The offset in bytes the compressed LLVM bytecode can be found at in format
// version 1
pub const RLIB_BYTECODE_OBJECT_V1_DATA_OFFSET: usize =
    RLIB_BYTECODE_OBJECT_V1_DATASIZE_OFFSET + 8;

fn extract_bytecode_format_version(bc: &[u8]) -> u32 {
    let pos = RLIB_BYTECODE_OBJECT_VERSION_OFFSET;
    let byte_data = &bc[pos..pos + 4];
    let data = unsafe { *(byte_data.as_ptr() as *const u32) };
    u32::from_le(data)
}

fn extract_compressed_bytecode_size_v1(bc: &[u8]) -> u64 {
    let pos = RLIB_BYTECODE_OBJECT_V1_DATASIZE_OFFSET;
    let byte_data = &bc[pos..pos + 8];
    let data = unsafe { *(byte_data.as_ptr() as *const u64) };
    u64::from_le(data)
}

fn main() {
	let path = env::args().nth(1).unwrap();

	let archive = ArchiveRO::open(&Path::new(&path)).expect("wanted an rlib");
    let bytecodes = archive.iter().filter_map(|child| {
        child.ok().and_then(|c| c.name().map(|name| (name, c)))
    }).filter(|&(name, _)| name.ends_with("bytecode.deflate"));
    for (name, data) in bytecodes {
        let bc_encoded = data.data();

        let bc_decoded = {
            // Read the version
            let version = extract_bytecode_format_version(bc_encoded);

            if version == 1 {
                // The only version existing so far
                let data_size = extract_compressed_bytecode_size_v1(bc_encoded);
                let compressed_data = &bc_encoded[
                    RLIB_BYTECODE_OBJECT_V1_DATA_OFFSET..
                    (RLIB_BYTECODE_OBJECT_V1_DATA_OFFSET + data_size as usize)];

                match flate::inflate_bytes(compressed_data) {
                    Ok(inflated) => inflated,
                    Err(_) => {
                        panic!("failed to decompress bc of `{}`",
                                           name)
                    }
                }
            } else {
                panic!("Unsupported bytecode format version {}",
                                   version)
            }
        };

        File::create(name).unwrap().write_all(&*bc_decoded).unwrap();
    }
}
