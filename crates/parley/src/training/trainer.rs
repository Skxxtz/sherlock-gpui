//! Averaged-perceptron training for both the intent rows and the tag rows.
//!
//! For each example: predict, and if wrong, add the example's features to
//! the correct row and subtract them from the predicted one. The final
//! weights are the average over all steps, which generalises much better
//! than the last state.

use super::Rng;
use super::corpus::Example;
use crate::features::{self, BUCKETS, Lexicon};
use crate::model::{Model, class_of};
use crate::{Tag, model};

/// Training settings.
#[derive(Debug, Clone)]
pub struct Settings {
    /// Passes over the data.
    pub epochs: usize,
    /// Share of examples that see the app lexicon, so the model learns
    /// to use it but still works without it.
    pub lexicon_rate: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            epochs: 10,
            lexicon_rate: 0.7,
        }
    }
}

/// One weight table with the bookkeeping needed for averaging.
struct Averaged {
    w: Vec<f32>,
    total: Vec<f32>,
    stamp: Vec<u32>,
}

impl Averaged {
    fn new(len: usize) -> Averaged {
        Averaged {
            w: vec![0.0; len],
            total: vec![0.0; len],
            stamp: vec![0; len],
        }
    }

    fn add(&mut self, row: usize, f: &[usize], d: f32, t: u32) {
        for &i in f {
            let k = row * BUCKETS + i;
            self.total[k] += (t - self.stamp[k]) as f32 * self.w[k];
            self.stamp[k] = t;
            self.w[k] += d;
        }
    }

    fn finish(mut self, t: u32) -> Vec<f32> {
        for k in 0..self.w.len() {
            self.total[k] += (t - self.stamp[k]) as f32 * self.w[k];
            self.w[k] = self.total[k] / t as f32;
        }
        self.w
    }
}

/// Train a model on `examples`.
pub fn train(examples: &[Example], lex: &Lexicon, s: &Settings, rng: &mut Rng) -> Model {
    let empty = Lexicon::default();
    let mut intent = Averaged::new(model::CLASSES * BUCKETS);
    let mut tags = Averaged::new(Tag::COUNT * BUCKETS);
    // the model used for predictions during training = current (not averaged) weights
    let mut live = Model::new();
    let mut f = Vec::with_capacity(32);
    let mut order: Vec<usize> = (0..examples.len()).collect();
    let mut t: u32 = 0;

    for _ in 0..s.epochs {
        for i in (1..order.len()).rev() {
            order.swap(i, rng.below(i + 1));
        }
        for &k in &order {
            let ex = &examples[k];
            let lex = if rng.chance(s.lexicon_rate) {
                lex
            } else {
                &empty
            };
            t += 1;

            // intent
            features::sentence(&ex.words, lex, &mut f);
            let scores = live.intent_scores(&f);
            let guess = (0..model::CLASSES)
                .max_by(|a, b| scores[*a].total_cmp(&scores[*b]))
                .unwrap();
            let gold = class_of(ex.intent);
            if guess != gold {
                intent.add(gold, &f, 1.0, t);
                intent.add(guess, &f, -1.0, t);
                sync(&mut live.intent, &intent.w, &[gold, guess], &f);
            }

            // tags, with the gold intent and gold previous tag
            let mut prev = Tag::O;
            for (i, &gold) in ex.tags.iter().enumerate() {
                features::token(&ex.words, i, prev, ex.intent, lex, &mut f);
                let guess = live.best_tag(prev, ex.intent, &f);
                if guess != gold {
                    tags.add(gold.index(), &f, 1.0, t);
                    tags.add(guess.index(), &f, -1.0, t);
                    sync(&mut live.tags, &tags.w, &[gold.index(), guess.index()], &f);
                }
                prev = gold;
            }
        }
    }
    Model {
        intent: intent.finish(t),
        tags: tags.finish(t),
    }
}

/// Copy the weights that just changed into the live model.
fn sync(live: &mut [f32], w: &[f32], rows: &[usize], f: &[usize]) {
    for &row in rows {
        for &i in f {
            let k = row * BUCKETS + i;
            live[k] = w[k];
        }
    }
}
