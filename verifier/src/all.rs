#![feature(log_syntax)]
#![feature(plugin)]
#![feature(const_fn)]

extern crate elfloader;
extern crate byteorder;
extern crate core;

use std::sync::Arc;
use std::sync::Mutex;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use elfloader::*;

mod decoder;
mod table;

use std::thread;

fn run_tests(o: u8, n: u8, f: &Mutex<File>) {
	let mut xs = Vec::new();
	xs.push(o);
	while xs.len() < 16 {
		xs.push(0);
	}
	while *xs.last().unwrap() != 255 {
		println!("testing({})) {:?}", o, &xs[..]);

		decoder::decode_test_allp(&xs, f);

		let n = xs[0].wrapping_add(n);

		if n < xs[0] { // overflow; increase next byte
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
	decoder::decode_test_allp(&xs, f);
}

fn main() {
	let n = 4;

	let file = Arc::new(Mutex::new(OpenOptions::new().create(true).write(true).append(true).open("errors").unwrap()));

    let mut threads = vec![];

	for i in 0..n {
		let t_file = file.clone();
	    threads.push(thread::spawn(move || {
	    	run_tests(i, n, &*t_file);
	    }));
	}

    for t in threads {
        t.join().unwrap();
    }
}
