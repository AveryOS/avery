#![feature(log_syntax)]
#![feature(plugin)]
#![feature(const_fn)]

extern crate elfloader;
extern crate byteorder;
extern crate core;
extern crate rand;

use std::sync::Arc;
use std::fs::OpenOptions;
use std::sync::Mutex;
use rand::Rng;
use std::fs::File;
use std::io::Read;
use elfloader::*;

mod decoder;
mod table;

use std::thread;

fn run_tests(o: u8, n: u8, f: &Mutex<File>) {
	let mut rnd = rand::thread_rng();
	let mut xs = Vec::new();
	while xs.len() < 16 {
		xs.push(0);
	}
	loop {
		rnd.fill_bytes(&mut xs);
		decoder::decode_test_allp(&xs, f);
	}
}

fn main() {
	thread::spawn(|| {
		std::thread::sleep(std::time::Duration::new(60 * 5, 0));
		let errors = decoder::FOUND_ERRORS.load(std::sync::atomic::Ordering::SeqCst);
	    println!("Terminating - found errors: {}", errors);
	    std::process::exit(if errors { -1 } else { 0 });
    });

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
