/// Returns the magnitude of a vector.
pub fn magnitude(a: &[f32]) -> f32 {
    libm::sqrtf(a.iter().map(|&x| x * x).sum::<f32>())
}

/// Returns the product of two vectors.
pub fn product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum()
}
