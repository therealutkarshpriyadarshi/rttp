//! Vector database for embeddings storage and similarity search
//!
//! Provides an in-memory vector store with cosine similarity search,
//! useful for RAG (Retrieval Augmented Generation) applications.

use std::collections::HashMap;

/// A single vector entry in the database
#[derive(Debug, Clone)]
pub struct VectorEntry {
    /// Unique identifier
    pub id: String,
    /// The embedding vector
    pub vector: Vec<f32>,
    /// Optional metadata
    pub metadata: HashMap<String, String>,
}

impl VectorEntry {
    /// Create a new vector entry
    pub fn new(id: impl Into<String>, vector: Vec<f32>) -> Self {
        Self {
            id: id.into(),
            vector,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the entry
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// In-memory vector store with similarity search
pub struct VectorStore {
    /// Stored vectors
    entries: Vec<VectorEntry>,
    /// Index mapping IDs to positions
    index: HashMap<String, usize>,
}

impl VectorStore {
    /// Create a new empty vector store
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Insert a vector into the store
    pub fn insert(&mut self, entry: VectorEntry) {
        let id = entry.id.clone();

        // Check if ID already exists
        if let Some(&pos) = self.index.get(&id) {
            // Update existing entry
            self.entries[pos] = entry;
        } else {
            // Add new entry
            let pos = self.entries.len();
            self.entries.push(entry);
            self.index.insert(id, pos);
        }
    }

    /// Insert a simple vector with just ID and vector
    pub fn insert_simple(&mut self, id: impl Into<String>, vector: Vec<f32>) {
        self.insert(VectorEntry::new(id, vector));
    }

    /// Get a vector by ID
    pub fn get(&self, id: &str) -> Option<&VectorEntry> {
        self.index.get(id).and_then(|&pos| self.entries.get(pos))
    }

    /// Remove a vector by ID
    pub fn remove(&mut self, id: &str) -> Option<VectorEntry> {
        if let Some(&pos) = self.index.get(id) {
            self.index.remove(id);

            // Remove from entries (swap with last element)
            let entry = if pos == self.entries.len() - 1 {
                self.entries.pop()
            } else {
                let last = self.entries.pop().unwrap();
                let removed = std::mem::replace(&mut self.entries[pos], last);

                // Update index for swapped element
                self.index.insert(self.entries[pos].id.clone(), pos);

                Some(removed)
            };

            entry
        } else {
            None
        }
    }

    /// Search for the top-k most similar vectors
    ///
    /// Returns a list of (id, similarity_score) tuples, sorted by similarity (highest first)
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<(String, f32)> {
        if self.entries.is_empty() {
            return Vec::new();
        }

        let mut scores: Vec<(String, f32)> = self
            .entries
            .iter()
            .map(|entry| {
                let similarity = cosine_similarity(query, &entry.vector);
                (entry.id.clone(), similarity)
            })
            .collect();

        // Sort by similarity (highest first)
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top-k results
        scores.into_iter().take(top_k).collect()
    }

    /// Search with a similarity threshold
    ///
    /// Returns only results with similarity >= threshold
    pub fn search_threshold(&self, query: &[f32], threshold: f32) -> Vec<(String, f32)> {
        if self.entries.is_empty() {
            return Vec::new();
        }

        let mut scores: Vec<(String, f32)> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let similarity = cosine_similarity(query, &entry.vector);
                if similarity >= threshold {
                    Some((entry.id.clone(), similarity))
                } else {
                    None
                }
            })
            .collect();

        // Sort by similarity (highest first)
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scores
    }

    /// Get the number of vectors in the store
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all vectors from the store
    pub fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
    }

    /// Get all vector IDs
    pub fn ids(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.id.clone()).collect()
    }

    /// Get all entries
    pub fn entries(&self) -> &[VectorEntry] {
        &self.entries
    }
}

impl Default for VectorStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate cosine similarity between two vectors
///
/// Returns a value between -1 and 1, where:
/// - 1 means vectors are identical
/// - 0 means vectors are orthogonal
/// - -1 means vectors are opposite
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    if a.is_empty() {
        return 0.0;
    }

    // Calculate dot product
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();

    // Calculate magnitudes
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    // Avoid division by zero
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }

    dot / (mag_a * mag_b)
}

/// Calculate euclidean distance between two vectors
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

/// Normalize a vector to unit length
pub fn normalize(vector: &mut [f32]) {
    let magnitude: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude > 0.0 {
        for x in vector.iter_mut() {
            *x /= magnitude;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 1.0];
        let b = vec![-1.0, -1.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 0.001);
    }

    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        let dist = euclidean_distance(&a, &b);
        assert!((dist - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_normalize() {
        let mut v = vec![3.0, 4.0];
        normalize(&mut v);

        let magnitude: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_vector_store_insert_get() {
        let mut store = VectorStore::new();

        let entry = VectorEntry::new("doc1", vec![1.0, 2.0, 3.0]);
        store.insert(entry);

        assert_eq!(store.len(), 1);

        let retrieved = store.get("doc1").unwrap();
        assert_eq!(retrieved.id, "doc1");
        assert_eq!(retrieved.vector, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_vector_store_update() {
        let mut store = VectorStore::new();

        store.insert(VectorEntry::new("doc1", vec![1.0, 2.0, 3.0]));
        store.insert(VectorEntry::new("doc1", vec![4.0, 5.0, 6.0]));

        assert_eq!(store.len(), 1);

        let retrieved = store.get("doc1").unwrap();
        assert_eq!(retrieved.vector, vec![4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_vector_store_remove() {
        let mut store = VectorStore::new();

        store.insert(VectorEntry::new("doc1", vec![1.0, 2.0, 3.0]));
        store.insert(VectorEntry::new("doc2", vec![4.0, 5.0, 6.0]));

        let removed = store.remove("doc1");
        assert!(removed.is_some());
        assert_eq!(store.len(), 1);
        assert!(store.get("doc1").is_none());
    }

    #[test]
    fn test_vector_store_search() {
        let mut store = VectorStore::new();

        store.insert(VectorEntry::new("doc1", vec![1.0, 0.0, 0.0]));
        store.insert(VectorEntry::new("doc2", vec![0.0, 1.0, 0.0]));
        store.insert(VectorEntry::new("doc3", vec![0.9, 0.1, 0.0]));

        let query = vec![1.0, 0.0, 0.0];
        let results = store.search(&query, 2);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "doc1"); // Most similar
        assert_eq!(results[1].0, "doc3"); // Second most similar
    }

    #[test]
    fn test_vector_store_search_threshold() {
        let mut store = VectorStore::new();

        store.insert(VectorEntry::new("doc1", vec![1.0, 0.0, 0.0]));
        store.insert(VectorEntry::new("doc2", vec![0.0, 1.0, 0.0]));
        store.insert(VectorEntry::new("doc3", vec![0.9, 0.1, 0.0]));

        let query = vec![1.0, 0.0, 0.0];
        let results = store.search_threshold(&query, 0.8);

        // Only doc1 and doc3 should meet the threshold
        assert!(results.len() >= 1);
        assert!(results.iter().all(|(_, score)| *score >= 0.8));
    }

    #[test]
    fn test_vector_entry_metadata() {
        let entry = VectorEntry::new("doc1", vec![1.0, 2.0, 3.0])
            .with_metadata("source", "article")
            .with_metadata("category", "tech");

        assert_eq!(entry.metadata.get("source"), Some(&"article".to_string()));
        assert_eq!(entry.metadata.get("category"), Some(&"tech".to_string()));
    }

    #[test]
    fn test_vector_store_clear() {
        let mut store = VectorStore::new();

        store.insert(VectorEntry::new("doc1", vec![1.0, 2.0, 3.0]));
        store.insert(VectorEntry::new("doc2", vec![4.0, 5.0, 6.0]));

        store.clear();

        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }
}
