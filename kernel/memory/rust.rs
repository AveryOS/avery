#![allow(unused_variables)]

use super::*;
use arch;
use std::ptr::copy_nonoverlapping;

#[no_mangle]
pub extern "C" fn __rust_allocate(size: usize, align: usize) -> *mut u8 {
    unsafe {
        let pages = div_up(PTR_BYTES + size + align, arch::PAGE_SIZE);
        let (block, page) = alloc_block(pages, Kind::Default);
        arch::memory::map(page, pages, arch::memory::RW_DATA_FLAGS);

        let p = page.ptr();

        *(p as *mut *mut Block) = block;

        (p + PTR_BYTES) as *mut u8
    }
}

#[no_mangle]
pub extern "C" fn __rust_deallocate(ptr: *mut u8, old_size: usize, align: usize) {
    unsafe {
        let block = *((ptr as usize - 8) as *const *mut Block);
        free_block(block);
    }
}

#[no_mangle]
pub extern "C" fn __rust_reallocate(ptr: *mut u8,
                                    old_size: usize,
                                    size: usize,
                                    align: usize)
                                    -> *mut u8 {
    unsafe { 
        let new = __rust_allocate(size, align);
        copy_nonoverlapping(ptr, new, old_size);
        __rust_deallocate(ptr, old_size, align);
        new
    }
}

#[no_mangle]
pub extern "C" fn __rust_reallocate_inplace(ptr: *mut u8,
                                            old_size: usize,
                                            size: usize,
                                            align: usize)
                                            -> usize {
    old_size
}

#[no_mangle]
pub extern "C" fn __rust_usable_size(size: usize, align: usize) -> usize {
    size
}
