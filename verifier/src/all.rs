#![feature(core)]
#![feature(trace_macros)]
#![feature(log_syntax)]
#![feature(plugin)]
//#![cfg_attr(test, feature(plugin, custom_attribute))]
//#![cfg_attr(test, plugin(quickcheck_macros))]

//#[cfg(test)]
//extern crate quickcheck;

extern crate elfloader;
extern crate byteorder;
extern crate core;

use std::fs::File;
use std::io::Read;
use elfloader::*;

mod decoder;
mod table;

use std::thread;

fn run_tests(o: u8, n: u8) {
	let mut xs = Vec::new();
	xs.push(o);
	while xs.len() < 16 {
		xs.push(0);
	}
	while *xs.last().unwrap() != 255 {
		decoder::decode_test_allp(&xs);

		let n = xs[0].wrapping_add(n);

		if n < xs[0] { // overflow; increase next byte
			println!("testing {:?}", &xs[..]);
			for j in 1..xs.len() {
				if xs[j] == 255 {
					xs[j] = 0;
				} else {
					xs[j] += 1;
					break;
				}
			}
		}

		xs[0] = n;
	}
	decoder::decode_test_allp(&xs);
}

fn main() {
	let n = 4;

    let mut threads = vec![];

	for i in 0..n {
	    threads.push(thread::spawn(move || {
	    	run_tests(i, n);
	    }));
	}

    for t in threads {
        t.join().unwrap();
    }
}
