/// A closed numeric domain `[min, max]` mapped onto `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale {
    pub min: f64,
    pub max: f64,
}

impl Scale {
    pub fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    /// Smallest domain covering every finite value. Empty input gives `0..1`,
    /// a single value gets ±1 (or ±10%) of padding so it doesn't collapse.
    pub fn fit(values: impl IntoIterator<Item = f64>, include_zero: bool) -> Self {
        let (mut min, mut max) = values
            .into_iter()
            .filter(|v| v.is_finite())
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            });
        if !min.is_finite() {
            return Self::new(0., 1.);
        }
        if include_zero {
            min = min.min(0.);
            max = max.max(0.);
        }
        if min == max {
            let pad = if min == 0. { 1. } else { min.abs() * 0.1 };
            min -= pad;
            max += pad;
        }
        Self::new(min, max)
    }

    pub fn span(&self) -> f64 {
        self.max - self.min
    }

    /// Position of `v` in the domain, `0.0` at `min`, `1.0` at `max`.
    pub fn norm(&self, v: f64) -> f64 {
        if self.span() == 0. {
            0.5
        } else {
            (v - self.min) / self.span()
        }
    }

    pub fn pad(self, amount: f64) -> Self {
        Self::new(self.min - amount, self.max + amount)
    }

    /// Widens the domain to multiples of a nice step and returns it with its ticks.
    pub fn nice(self, count: usize) -> (Self, Vec<f64>, f64) {
        let step = nice_step(self.span(), count.max(1));
        let min = (self.min / step).floor() * step;
        let max = (self.max / step).ceil() * step;
        (Self::new(min, max), ticks_in(min, max, step), step)
    }

    /// Ticks at nice steps strictly inside the domain, domain left unchanged.
    pub fn ticks(&self, count: usize) -> (Vec<f64>, f64) {
        let step = nice_step(self.span(), count.max(1));
        let first = (self.min / step).ceil() * step;
        (ticks_in(first, self.max, step), step)
    }
}

fn ticks_in(from: f64, to: f64, step: f64) -> Vec<f64> {
    let n = ((to - from) / step + 1e-9).floor() as usize;
    (0..=n)
        .map(|i| from + i as f64 * step)
        // snap float noise like 0.30000000000000004
        .map(|v| (v / step).round() * step)
        .collect()
}

/// A step of 1, 2, 2.5 or 5 × 10ⁿ giving roughly `count` intervals over `span`.
pub fn nice_step(span: f64, count: usize) -> f64 {
    if !(span.is_finite() && span > 0.) {
        return 1.;
    }
    let raw = span / count as f64;
    let mag = 10f64.powf(raw.log10().floor());
    let frac = raw / mag;
    let nice = match frac {
        f if f <= 1. => 1.,
        f if f <= 2. => 2.,
        f if f <= 2.5 => 2.5,
        f if f <= 5. => 5.,
        _ => 10.,
    };
    nice * mag
}

/// Formats `v` with just enough decimals for `step`; large values get k/M/G suffixes.
pub fn format_tick(v: f64, step: f64) -> String {
    let abs = v.abs();
    let (div, suffix) = match abs {
        a if a >= 1e9 => (1e9, "G"),
        a if a >= 1e6 => (1e6, "M"),
        a if a >= 1e4 => (1e3, "k"),
        _ => (1., ""),
    };
    let scaled_step = step / div;
    let decimals = if scaled_step >= 1. || scaled_step <= 0. {
        0
    } else {
        (-scaled_step.log10().floor()) as usize + usize::from(is_quarter(scaled_step))
    };
    let s = format!("{:.*}", decimals.min(6), v / div);
    // avoid "-0"
    let s = if s.trim_start_matches(['-', '0', '.']).is_empty() {
        s.trim_start_matches('-').to_string()
    } else {
        s
    };
    format!("{s}{suffix}")
}

/// 0.25, 0.025 … need one more decimal than their magnitude suggests.
fn is_quarter(step: f64) -> bool {
    let mag = 10f64.powf(step.log10().floor());
    ((step / mag) - 2.5).abs() < 1e-9
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_handles_edge_cases() {
        assert_eq!(Scale::fit([], false), Scale::new(0., 1.));
        assert_eq!(Scale::fit([f64::NAN, 3., 7.], false), Scale::new(3., 7.));
        assert_eq!(Scale::fit([5., 5.], false), Scale::new(4.5, 5.5));
        assert_eq!(Scale::fit([0.], false), Scale::new(-1., 1.));
        assert_eq!(Scale::fit([3., 7.], true), Scale::new(0., 7.));
    }

    #[test]
    fn nice_steps() {
        assert_eq!(nice_step(10., 5), 2.);
        assert_eq!(nice_step(1., 4), 0.25);
        assert_eq!(nice_step(730., 4), 200.);
        assert_eq!(nice_step(0., 4), 1.);
    }

    #[test]
    fn nice_extends_domain() {
        let (s, ticks, step) = Scale::new(3., 97.).nice(4);
        assert_eq!(step, 25.);
        assert_eq!(s, Scale::new(0., 100.));
        assert_eq!(ticks, vec![0., 25., 50., 75., 100.]);
    }

    #[test]
    fn ticks_stay_inside() {
        let (ticks, _) = Scale::new(0.1, 0.95).ticks(4);
        assert_eq!(ticks, vec![0.25, 0.5, 0.75]);
    }

    #[test]
    fn formatting() {
        assert_eq!(format_tick(20., 5.), "20");
        assert_eq!(format_tick(0.5, 0.25), "0.50");
        assert_eq!(format_tick(0.2, 0.2), "0.2");
        assert_eq!(format_tick(25_000., 5_000.), "25k");
        assert_eq!(format_tick(1_500_000., 500_000.), "1.5M");
        assert_eq!(format_tick(-0.0001, 0.5), "0.0");
    }
}
