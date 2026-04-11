/// Filter out short-term fluctuations in signals.
pub struct LowpassFilter {
    /// Gain.
    k: f32,
    /// Current output value.
    output: f32,
}

impl LowpassFilter {
    /// Create a new lowpass filter with the given gain.
    pub fn new(k: f32) -> Self {
        Self { k, output: 0.0 }
    }

    /// Set the gain.
    pub fn set_gain(&mut self, k: f32) {
        self.k = k;
    }

    /// Filter a measurement and return the updated output.
    pub fn filter(&mut self, measurement: f32) -> f32 {
        self.output = (self.k * self.output) + ((1.0 - self.k) * measurement);
        self.output
    }
}
