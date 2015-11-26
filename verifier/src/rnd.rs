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
extern crate rand;

use rand::Rng;
use std::fs::File;
use std::io::Read;
use elfloader::*;

mod decoder;
mod table;

use std::thread;

fn run_tests(o: u8, n: u8) {
	let mut rnd = rand::thread_rng();
	let mut xs = Vec::new();
	while xs.len() < 16 {
		xs.push(0);
	}
	loop {
		rnd.fill_bytes(&mut xs);
		decoder::decode_test_allp(&xs);
	}
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
