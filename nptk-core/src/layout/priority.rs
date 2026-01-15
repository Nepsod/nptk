// SPDX-License-Identifier: LGPL-3.0-only

/// Calculate flex_grow adjustment based on layout priority.
///
/// Higher priority widgets should get more space, so we convert priority
/// to a flex_grow multiplier. The base flex_grow is multiplied by (1 + priority).
///
/// # Example
///
/// - priority = 0.0: flex_grow unchanged (multiplier = 1.0)
/// - priority = 1.0: flex_grow doubled (multiplier = 2.0)
/// - priority = 2.0: flex_grow tripled (multiplier = 3.0)
pub fn adjust_flex_grow_for_priority(base_flex_grow: f32, priority: f32) -> f32 {
    if priority <= 0.0 {
        base_flex_grow
    } else {
        base_flex_grow * (1.0 + priority)
    }
}

/// Calculate flex_shrink adjustment based on layout priority.
///
/// Higher priority widgets should shrink less, so we convert priority
/// to a flex_shrink divisor. Higher priority = lower flex_shrink.
///
/// # Example
///
/// - priority = 0.0: flex_shrink unchanged (divisor = 1.0)
/// - priority = 1.0: flex_shrink halved (divisor = 2.0)
/// - priority = 2.0: flex_shrink divided by 3 (divisor = 3.0)
pub fn adjust_flex_shrink_for_priority(base_flex_shrink: f32, priority: f32) -> f32 {
    if priority <= 0.0 {
        base_flex_shrink
    } else {
        base_flex_shrink / (1.0 + priority)
    }
}

/// Normalize priorities across a set of widgets.
///
/// This ensures that priorities are relative to each other, making
/// the priority system more predictable. The highest priority becomes 1.0,
/// and others are scaled proportionally.
///
/// # Returns
///
/// A vector of normalized priorities, where the maximum priority is 1.0.
pub fn normalize_priorities(priorities: &[f32]) -> Vec<f32> {
    if priorities.is_empty() {
        return vec![];
    }

    let max_priority = priorities.iter().copied().fold(0.0f32, f32::max);
    
    if max_priority <= 0.0 {
        // All priorities are zero or negative, return as-is
        priorities.to_vec()
    } else {
        // Normalize so max priority is 1.0
        priorities.iter().map(|&p| p / max_priority).collect()
    }
}
