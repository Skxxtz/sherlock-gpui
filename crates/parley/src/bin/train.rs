//! Trains the model and writes the weights file embedded by the library.
//! `cargo run -p parley --features train --bin train --release`
//!
//! For now: loads the data files and prints statistics and samples.

use parley::train::Rng;
use parley::train::data::{Dataset, Split};

fn main() {
    let ds = match Dataset::bundled() {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };
    println!(
        "templates: {} train, {} held out",
        ds.template_count(Split::Train),
        ds.template_count(Split::Test)
    );

    let mut rng = Rng::new(42);
    let train = ds.generate(Split::Train, Split::Train, 20_000, &mut rng, true);
    let mut skeletons: Vec<_> = train.iter().map(|e| e.skeleton()).collect();
    skeletons.sort();
    skeletons.dedup();
    println!(
        "20000 training examples → {} distinct phrasings\n",
        skeletons.len()
    );

    println!("samples (training):");
    for ex in train.iter().take(12) {
        let intent = ex.intent.map_or("none", |i| i.name());
        println!("  {intent:<7} {:<44} {}", ex.text(), ex.skeleton());
    }
    println!("\nsamples (held out phrasings + values):");
    for ex in ds.generate(Split::Test, Split::Test, 6, &mut rng, false) {
        let intent = ex.intent.map_or("none", |i| i.name());
        println!("  {intent:<7} {:<44} {}", ex.text(), ex.skeleton());
    }
}
