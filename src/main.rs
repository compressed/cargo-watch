//! Watch files in a Cargo project and compile it when they change

extern crate rustc_serialize;
extern crate docopt;
#[no_link] extern crate docopt_macros;

extern crate notify;
#[macro_use] extern crate log;
extern crate env_logger;

use docopt::Docopt;
use notify::{Error, RecommendedWatcher, Watcher};
use std::sync::mpsc::channel;
use std::sync::Arc;

mod cargo;
mod compile;
mod ignore;
mod timelock;

static USAGE: &'static str = "
Usage: cargo-watch [watch] [options]
       cargo watch [options]

Options:
  -h, --help      Display this message
  -b, --build     Run `cargo build` when a file is modified
  -d, --doc       Run `cargo doc` when a file is modified
  -t, --test      Run `cargo test` when a file is modified
  -n, --bench     Run `cargo bench` when a file is modified

Default options are `build` and `test`
";

#[derive(RustcDecodable, Debug)]
struct Args {
    flag_build: bool,
    flag_doc: bool,
    flag_test: bool,
    flag_bench: bool,
}

#[derive(Clone,Copy)]
pub struct Config {
    build: bool,
    doc: bool,
    test: bool,
    bench: bool
}

impl Config {
  fn new() -> Config {
    #![allow(unused_variables)]
    let Args {
      flag_build: mut build,
      flag_doc: doc,
      flag_test: mut test,
      flag_bench: bench,
    } = Docopt::new(USAGE).and_then(|d| d.decode()).unwrap_or_else(|e| e.exit());

    if !build && !doc &&
      !test && !bench {
        // Default to build & doc
        build = true;
        test = true;
      }

    Config {
      build: build,
      doc: doc,
      test: test,
      bench: bench
    }
  }
}

fn main() {
  env_logger::init().unwrap();
  let config = Config::new();
  let (tx, rx) = channel();
  let w: Result<RecommendedWatcher, Error> = Watcher::new(tx);
  let mut watcher = match w {
    Ok(i) => i,
    Err(_) => {
      error!("Failed to init notify");
      std::process::exit(1);
    }
  };

  let t = timelock::new();
  let c = Arc::new(config);
  match cargo::root() {
    Some(p) => {
      let _ = watcher.watch(&p.join("src"));
      let _ = watcher.watch(&p.join("tests"));
      let _ = watcher.watch(&p.join("benches"));

      loop {
        match rx.recv() {
          Ok(e) => compile::handle_event(&t, e, c.clone()),
          Err(_) => ()
        }
      }
    },
    None => {
      error!("Not a Cargo project, aborting.");
      std::process::exit(64);
    }
  }
}
