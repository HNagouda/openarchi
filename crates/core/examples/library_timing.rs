//! Times what the app does with a library on every save and every gallery
//! visit: the project list, the check whether someone else saved a project,
//! and a save with the scene already packed, as the app saves after an edit.
//! Saves go into the library's backups, so point it at a copy.
//!
//! cargo run --release -p openarchi-core --example library_timing -- <library copy>

use std::path::Path;
use std::time::Instant;

use openarchi_core::library::Library;
use openarchi_core::project::packed_scene;

/// Each measure is run this many times and the fastest kept, so a busy machine does not skew it.
const RUNS: usize = 5;

fn main() {
    let root = std::env::args()
        .nth(1)
        .expect("usage: library_timing <library copy>");
    let library = Library::at(Path::new(&root)).expect("open the library");
    report("list", || {
        library.list().expect("list the projects");
    });

    for summary in library.list().expect("list the projects") {
        let mut project = library.load(&summary.id).expect("load the project");
        let packed = project
            .scene
            .take()
            .map(|scene| packed_scene::pack(&scene).expect("pack the scene"));
        println!("{}", summary.id);
        report("modified", || {
            library.modified(&summary.id).expect("read modified");
        });
        report("save", || {
            library.modified(&summary.id).expect("read modified");
            match &packed {
                Some(packed) => library.save_with_packed_scene(&mut project, packed),
                None => library.save(&mut project),
            }
            .expect("save the project");
        });
    }
}

fn report(name: &str, mut work: impl FnMut()) {
    let fastest = (0..RUNS)
        .map(|_| {
            let started = Instant::now();
            work();
            started.elapsed()
        })
        .min()
        .unwrap_or_default();
    println!("  {name:<10} {fastest:?}");
}
