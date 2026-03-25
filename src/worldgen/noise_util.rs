use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

/// Thin wrapper around `Fbm<Perlin>` that standardizes noise sampling for worldgen.
///
/// All configuration happens at construction; callers just call `sample` or
/// `sample_normalized` with world-space coordinates.
#[derive(Clone)]
pub struct NoiseLayer {
    fbm: Fbm<Perlin>,
}

impl NoiseLayer {
    /// Create a new noise layer with the given seed, base frequency, and octave count.
    ///
    /// - `frequency`: controls feature scale. Lower = larger features.
    ///   E.g. 0.01 on a 256-wide map → features spanning ~25-40 cells.
    /// - `octaves`: number of noise layers blended (detail levels). 4-8 is typical.
    pub fn new(seed: u32, frequency: f64, octaves: usize) -> Self {
        let fbm = Fbm::<Perlin>::new(seed)
            .set_frequency(frequency)
            .set_octaves(octaves)
            .set_lacunarity(2.0)
            .set_persistence(0.5);
        Self { fbm }
    }

    /// Sample raw noise at (x, y). Returns approximately -1.0 to 1.0,
    /// though Fbm can occasionally exceed this range slightly.
    pub fn sample(&self, x: f64, y: f64) -> f64 {
        self.fbm.get([x, y])
    }

    /// Sample noise at (x, y) and map to 0.0..1.0.
    /// Values are clamped so the result is always in [0.0, 1.0].
    pub fn sample_normalized(&self, x: f64, y: f64) -> f64 {
        ((self.sample(x, y) + 1.0) * 0.5).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_same_seed() {
        let a = NoiseLayer::new(42, 0.01, 6);
        let b = NoiseLayer::new(42, 0.01, 6);
        for i in 0..50 {
            let x = i as f64 * 1.7;
            let y = i as f64 * 0.9;
            assert_eq!(a.sample(x, y), b.sample(x, y));
        }
    }

    #[test]
    fn different_seeds_differ() {
        let a = NoiseLayer::new(1, 0.01, 6);
        let b = NoiseLayer::new(2, 0.01, 6);
        let mut diffs = 0;
        for i in 0..50 {
            let x = i as f64 * 1.7;
            let y = i as f64 * 0.9;
            if (a.sample(x, y) - b.sample(x, y)).abs() > 1e-10 {
                diffs += 1;
            }
        }
        assert!(diffs > 40, "Different seeds should produce mostly different values");
    }

    #[test]
    fn normalized_in_range() {
        let layer = NoiseLayer::new(99, 0.05, 6);
        for xi in 0..100 {
            for yi in 0..100 {
                let v = layer.sample_normalized(xi as f64, yi as f64);
                assert!(
                    (0.0..=1.0).contains(&v),
                    "sample_normalized out of range: {v} at ({xi}, {yi})"
                );
            }
        }
    }
}
