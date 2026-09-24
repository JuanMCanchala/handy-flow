//! Pure agglomerative clustering of speaker embeddings into speaker labels.
//! No I/O, no ONNX, no Tauri types — callers own embedding extraction and
//! model inference. Kept separate so the clustering logic is unit-testable
//! on synthetic embeddings without any model files on disk.

/// Cosine distance between two equal-length embeddings: `1 - cosine_similarity`.
/// Zero vectors are treated as maximally dissimilar from anything (distance 1.0)
/// since cosine similarity is undefined for them.
pub fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "embeddings must have equal length");

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return 1.0;
    }

    let similarity = dot / (norm_a.sqrt() * norm_b.sqrt());
    1.0 - similarity.clamp(-1.0, 1.0)
}

/// Assign a 0-based speaker index to each embedding using average-linkage
/// agglomerative clustering on cosine distance. Clusters are merged greedily
/// (closest pair first) as long as the closest pair's distance stays below
/// `threshold`; merging stops once no pair is closer than `threshold`.
///
/// Cluster indices are assigned in order of each cluster's earliest member,
/// so `result[i]` is stable and speaker 0 is whoever's segment appears first.
///
/// Returns an empty vec for an empty input. A single embedding always maps
/// to speaker 0.
pub fn cluster_embeddings(embeddings: &[Vec<f32>], threshold: f32) -> Vec<usize> {
    let n = embeddings.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![0];
    }

    // Each cluster starts as a single embedding's index; `members[c]` holds
    // the original indices belonging to cluster `c`.
    let mut members: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();
    // `alive[c]` is false once cluster `c` has been merged into another.
    let mut alive = vec![true; n];

    loop {
        // Find the closest pair of distinct alive clusters by average-linkage
        // distance (mean pairwise cosine distance between their members).
        let mut best: Option<(usize, usize, f32)> = None;
        let alive_indices: Vec<usize> = (0..members.len()).filter(|&c| alive[c]).collect();

        for (pos, &i) in alive_indices.iter().enumerate() {
            for &j in &alive_indices[pos + 1..] {
                let dist = average_linkage_distance(embeddings, &members[i], &members[j]);
                if best.is_none_or(|(_, _, best_dist)| dist < best_dist) {
                    best = Some((i, j, dist));
                }
            }
        }

        match best {
            Some((i, j, dist)) if dist < threshold => {
                // Merge j into i, keep i alive.
                let moved = std::mem::take(&mut members[j]);
                members[i].extend(moved);
                alive[j] = false;
            }
            _ => break,
        }
    }

    // Stable speaker numbering: order surviving clusters by their earliest
    // original member index, then map every embedding to that speaker id.
    let mut surviving: Vec<usize> = (0..members.len()).filter(|&c| alive[c]).collect();
    surviving.sort_by_key(|&c| members[c].iter().copied().min().unwrap_or(usize::MAX));

    let mut labels = vec![0usize; n];
    for (speaker_id, &cluster) in surviving.iter().enumerate() {
        for &member in &members[cluster] {
            labels[member] = speaker_id;
        }
    }
    labels
}

/// Mean cosine distance between every pair of embeddings drawn one from each
/// side. O(|a| * |b|); clusters stay small in practice (one entry per speech
/// segment in a single recording), so this is not a bottleneck.
fn average_linkage_distance(embeddings: &[Vec<f32>], a: &[usize], b: &[usize]) -> f32 {
    let mut sum = 0.0f32;
    let mut count = 0u32;
    for &i in a {
        for &j in b {
            sum += cosine_distance(&embeddings[i], &embeddings[j]);
            count += 1;
        }
    }
    if count == 0 {
        f32::MAX
    } else {
        sum / count as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn embedding(seed: &[f32]) -> Vec<f32> {
        seed.to_vec()
    }

    #[test]
    fn cosine_distance_is_zero_for_identical_vectors() {
        let a = embedding(&[1.0, 2.0, 3.0]);
        assert!(cosine_distance(&a, &a).abs() < 1e-6);
    }

    #[test]
    fn cosine_distance_is_two_for_opposite_vectors() {
        let a = embedding(&[1.0, 0.0]);
        let b = embedding(&[-1.0, 0.0]);
        assert!((cosine_distance(&a, &b) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_distance_is_one_for_orthogonal_vectors() {
        let a = embedding(&[1.0, 0.0]);
        let b = embedding(&[0.0, 1.0]);
        assert!((cosine_distance(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_distance_handles_zero_vectors() {
        let zero = embedding(&[0.0, 0.0]);
        let other = embedding(&[1.0, 1.0]);
        assert_eq!(cosine_distance(&zero, &other), 1.0);
        assert_eq!(cosine_distance(&zero, &zero), 1.0);
    }

    #[test]
    fn empty_input_returns_empty_labels() {
        let embeddings: Vec<Vec<f32>> = Vec::new();
        assert_eq!(cluster_embeddings(&embeddings, 0.5), Vec::<usize>::new());
    }

    #[test]
    fn single_embedding_is_speaker_zero() {
        let embeddings = vec![embedding(&[1.0, 0.0, 0.0])];
        assert_eq!(cluster_embeddings(&embeddings, 0.5), vec![0]);
    }

    #[test]
    fn two_similar_embeddings_merge_into_one_speaker() {
        let embeddings = vec![embedding(&[1.0, 0.0, 0.0]), embedding(&[0.99, 0.01, 0.0])];
        let labels = cluster_embeddings(&embeddings, 0.1);
        assert_eq!(labels[0], labels[1]);
    }

    #[test]
    fn two_dissimilar_embeddings_stay_separate_speakers() {
        let embeddings = vec![embedding(&[1.0, 0.0, 0.0]), embedding(&[0.0, 1.0, 0.0])];
        let labels = cluster_embeddings(&embeddings, 0.1);
        assert_ne!(labels[0], labels[1]);
    }

    #[test]
    fn two_speaker_alternating_conversation_recovers_two_clusters() {
        // Simulate an alternating conversation between two distinct voices,
        // each with small per-segment jitter around its centroid.
        let speaker_a = [1.0f32, 0.05, 0.0];
        let speaker_b = [0.0f32, 0.05, 1.0];
        let embeddings = vec![
            embedding(&speaker_a),
            embedding(&speaker_b),
            embedding(&[1.02, 0.04, 0.01]),
            embedding(&[0.01, 0.06, 0.98]),
            embedding(&[0.98, 0.06, -0.01]),
            embedding(&[-0.01, 0.05, 1.01]),
        ];

        let labels = cluster_embeddings(&embeddings, 0.15);

        assert_eq!(labels.len(), 6);
        // Segments 0, 2, 4 are speaker A's; 1, 3, 5 are speaker B's.
        assert_eq!(labels[0], labels[2]);
        assert_eq!(labels[0], labels[4]);
        assert_eq!(labels[1], labels[3]);
        assert_eq!(labels[1], labels[5]);
        assert_ne!(labels[0], labels[1]);
        // Exactly two distinct speakers were found.
        let unique: std::collections::HashSet<usize> = labels.iter().copied().collect();
        assert_eq!(unique.len(), 2);
    }

    #[test]
    fn speaker_ids_are_ordered_by_first_appearance() {
        // Speaker B appears first (index 0), speaker A second (index 1).
        let speaker_a = [1.0f32, 0.0, 0.0];
        let speaker_b = [0.0f32, 1.0, 0.0];
        let embeddings = vec![
            embedding(&speaker_b),
            embedding(&speaker_a),
            embedding(&[0.01, 0.99, 0.0]), // speaker B again
        ];

        let labels = cluster_embeddings(&embeddings, 0.1);

        assert_eq!(labels[0], 0); // first appearance -> speaker 0
        assert_eq!(labels[2], 0); // same voice as segment 0
        assert_eq!(labels[1], 1); // distinct voice -> speaker 1
    }

    #[test]
    fn three_speakers_with_tight_threshold_stay_separate() {
        let embeddings = vec![
            embedding(&[1.0, 0.0, 0.0]),
            embedding(&[0.0, 1.0, 0.0]),
            embedding(&[0.0, 0.0, 1.0]),
        ];
        let labels = cluster_embeddings(&embeddings, 0.05);
        let unique: std::collections::HashSet<usize> = labels.iter().copied().collect();
        assert_eq!(unique.len(), 3);
    }

    #[test]
    fn very_high_threshold_merges_everything_into_one_speaker() {
        let embeddings = vec![
            embedding(&[1.0, 0.0, 0.0]),
            embedding(&[0.0, 1.0, 0.0]),
            embedding(&[0.0, 0.0, 1.0]),
        ];
        let labels = cluster_embeddings(&embeddings, 3.0);
        let unique: std::collections::HashSet<usize> = labels.iter().copied().collect();
        assert_eq!(unique.len(), 1);
    }

    #[test]
    fn very_low_threshold_keeps_every_embedding_distinct() {
        let embeddings = vec![
            embedding(&[1.0, 0.0, 0.0]),
            embedding(&[1.0, 0.0, 0.0]),
            embedding(&[1.0, 0.0, 0.0]),
        ];
        // A threshold of 0.0 never merges (distance must be strictly < threshold).
        let labels = cluster_embeddings(&embeddings, 0.0);
        let unique: std::collections::HashSet<usize> = labels.iter().copied().collect();
        assert_eq!(unique.len(), 3);
    }
}
