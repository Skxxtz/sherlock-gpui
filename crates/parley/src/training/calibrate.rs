//! Fits the softmax temperature so `confidence` means what it says.
//!
//! Uses k-fold cross-validation over the *training* templates: train on
//! k-1 folds, score the unseen fold, repeat. The temperature that gives
//! those unseen phrasings the most accurate probabilities (lowest
//! negative log-likelihood of the right intent) is the one we keep.
//! The test split is never touched.

use super::Rng;
use super::corpus::{Dataset, Split};
use super::trainer::{Settings, train};
use crate::features::{self, Lexicon};
use crate::model::{CLASSES, class_of};

/// Folds for cross-validation.
pub const FOLDS: usize = 5;

/// Fit the temperature; returns it and the held-out NLL at T = 1 and at the fit.
pub fn temperature(
    ds: &Dataset,
    lex: &Lexicon,
    s: &Settings,
    n: usize,
    rng: &mut Rng,
) -> (f32, f64, f64) {
    // (scores, gold class) for phrasings the scoring model never saw
    let mut held_out: Vec<([f32; CLASSES], usize)> = Vec::new();
    let mut f = Vec::new();
    for i in 0..FOLDS {
        let (rest, fold) = ds.fold(FOLDS, i);
        let model = train(
            &rest.generate(Split::Train, Split::Train, n, rng, true),
            lex,
            s,
            rng,
        );
        for ex in fold.generate(Split::Train, Split::Train, n / 4, rng, false) {
            features::sentence(&ex.words, lex, &mut f);
            held_out.push((model.intent_scores(&f), class_of(ex.intent)));
        }
    }

    let nll = |t: f32| -> f64 {
        let total: f64 = held_out
            .iter()
            .map(|(s, gold)| {
                let max = s.iter().copied().fold(f32::MIN, f32::max);
                let z: f64 = s.iter().map(|v| (((v - max) / t) as f64).exp()).sum();
                -((((s[*gold] - max) / t) as f64).exp() / z).max(1e-12).ln()
            })
            .sum();
        total / held_out.len() as f64
    };

    // log-spaced search, then refine around the best
    let mut best = (1.0f32, nll(1.0));
    for k in -20..=40 {
        let t = 10f32.powf(k as f32 / 10.0);
        let v = nll(t);
        if v < best.1 {
            best = (t, v);
        }
    }
    for k in -10..=10 {
        let t = best.0 * 10f32.powf(k as f32 / 100.0);
        let v = nll(t);
        if v < best.1 {
            best = (t, v);
        }
    }
    (best.0, nll(1.0), best.1)
}
