//! Vector normalization utilities.

/// L2 normalize a vector (unit length).
pub fn normalize_l2(vector: &mut [f32]) {
    let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in vector.iter_mut() {
            *v /= norm;
        }
    }
}

/// L1 normalize a vector (sum to 1).
pub fn normalize_l1(vector: &mut [f32]) {
    let sum: f32 = vector.iter().map(|x| x.abs()).sum();
    if sum > 0.0 {
        for v in vector.iter_mut() {
            *v /= sum;
        }
    }
}

/// Min-max normalize to [0, 1] range.
pub fn normalize_minmax(vector: &mut [f32]) {
    let min = vector.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = vector.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let range = max - min;

    if range > 0.0 {
        for v in vector.iter_mut() {
            *v = (*v - min) / range;
        }
    }
}

/// Z-score normalize (mean=0, std=1).
pub fn normalize_zscore(vector: &mut [f32]) {
    let n = vector.len() as f32;
    let mean: f32 = vector.iter().sum::<f32>() / n;
    let variance: f32 = vector.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / n;
    let std = variance.sqrt();

    if std > 0.0 {
        for v in vector.iter_mut() {
            *v = (*v - mean) / std;
        }
    }
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

/// Compute Euclidean distance between two vectors.
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::INFINITY;
    }

    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

/// Compute dot product between two vectors.
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_l2_normalize() {
        let mut v = vec![3.0, 4.0];
        normalize_l2(&mut v);
        assert!((v[0] - 0.6).abs() < 0.001);
        assert!((v[1] - 0.8).abs() < 0.001);

        // Check unit length
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0];
        let c = vec![0.0, 1.0];
        let d = vec![-1.0, 0.0];

        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001); // Same direction
        assert!(cosine_similarity(&a, &c).abs() < 0.001); // Orthogonal
        assert!((cosine_similarity(&a, &d) + 1.0).abs() < 0.001); // Opposite
    }

    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];

        assert!((euclidean_distance(&a, &b) - 5.0).abs() < 0.001);
        assert!(euclidean_distance(&a, &a) < 0.001);
    }
}
