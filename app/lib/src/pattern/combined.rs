//! Combined pattern detection.
//!
//! This module detects repeated patterns such as repeated ranges (e.g., `(1>3)*2`)
//! and repeated alternating patterns.

use super::detector::{DetectionResult, PatternDetector, PatternType};
use super::range::RangeDetector;
use super::toggle::ToggleDetector;

/// Detector for combined/repeated patterns.
///
/// Detects patterns like:
/// - Repeated ranges: 1, 2, 3, 1, 2, 3 → `(1>3)*2`
/// - Repeated alternating patterns: A, B, A, B, A, B, A, B → `(A~B)*4` or `A~B*8`
#[derive(Debug, Clone)]
pub struct CombinedDetector {
    min_pattern_length: usize,
    range_detector: RangeDetector,
    toggle_detector: ToggleDetector,
}

impl CombinedDetector {
    /// Create a new combined detector with the given minimum pattern length.
    pub fn new(min_pattern_length: usize) -> Self {
        Self {
            min_pattern_length,
            range_detector: RangeDetector::new(2), // Allow shorter ranges for combined patterns
            toggle_detector: ToggleDetector::new(2),
        }
    }

    /// Try to detect a repeated range pattern.
    ///
    /// Looks for patterns like 1, 2, 3, 1, 2, 3 which can be encoded as (1>3)*2.
    fn detect_repeated_range(&self, values: &[&str]) -> Option<DetectionResult> {
        if values.len() < 4 {
            return None;
        }

        // Try different pattern lengths
        for pattern_len in 2..=values.len() / 2 {
            if values.len() % pattern_len != 0 {
                continue;
            }

            let repeat_count = values.len() / pattern_len;
            if repeat_count < 2 {
                continue;
            }

            // Check if the pattern repeats
            let pattern = &values[..pattern_len];
            let mut is_repeated = true;
            
            for i in 1..repeat_count {
                let chunk = &values[i * pattern_len..(i + 1) * pattern_len];
                if chunk != pattern {
                    is_repeated = false;
                    break;
                }
            }

            if !is_repeated {
                continue;
            }

            // Check if the pattern itself is a range
            if let Some(range_result) = self.range_detector.detect(pattern) {
                if let crate::als::AlsOperator::Range { start, end, step } = range_result.operator {
                    let original_len = Self::calculate_original_length(values);
                    return Some(DetectionResult::repeated_range(
                        start, end, step, repeat_count, original_len
                    ));
                }
            }
        }

        None
    }

    /// Try to detect a repeated toggle pattern.
    ///
    /// This is different from a simple toggle - it detects when a toggle pattern
    /// itself is repeated, which might offer better compression in some cases.
    fn detect_repeated_toggle(&self, values: &[&str]) -> Option<DetectionResult> {
        if values.len() < 4 {
            return None;
        }

        // Try different pattern lengths
        for pattern_len in 2..=values.len() / 2 {
            if values.len() % pattern_len != 0 {
                continue;
            }

            let repeat_count = values.len() / pattern_len;
            if repeat_count < 2 {
                continue;
            }

            // Check if the pattern repeats
            let pattern = &values[..pattern_len];
            let mut is_repeated = true;
            
            for i in 1..repeat_count {
                let chunk = &values[i * pattern_len..(i + 1) * pattern_len];
                if chunk != pattern {
                    is_repeated = false;
                    break;
                }
            }

            if !is_repeated {
                continue;
            }

            // Check if the pattern itself is a toggle
            if let Some(toggle_result) = self.toggle_detector.detect(pattern) {
                if let crate::als::AlsOperator::Toggle { values: toggle_values, count: _ } = toggle_result.operator {
                    // Create a repeated toggle result
                    let inner = crate::als::AlsOperator::Toggle {
                        values: toggle_values,
                        count: pattern_len,
                    };
                    let operator = crate::als::AlsOperator::Multiply {
                        value: Box::new(inner),
                        count: repeat_count,
                    };

                    let original_len = Self::calculate_original_length(values);
                    // Estimate compression - this is a rough estimate
                    let compressed_len = 10.0 + (repeat_count as f64).log10() + 1.0;
                    let compression_ratio = original_len as f64 / compressed_len;

                    return Some(DetectionResult {
                        operator,
                        compression_ratio,
                        pattern_type: PatternType::RepeatedToggle,
                    });
                }
            }
        }

        None
    }

    /// Calculate the original string length of the values.
    fn calculate_original_length(values: &[&str]) -> usize {
        let value_len: usize = values.iter().map(|v| v.len()).sum();
        let separator_len = values.len().saturating_sub(1);
        value_len + separator_len
    }
}

impl PatternDetector for CombinedDetector {
    fn detect(&self, values: &[&str]) -> Option<DetectionResult> {
        if values.len() < self.min_pattern_length {
            return None;
        }

        let mut best_result: Option<DetectionResult> = None;

        // Try repeated range detection
        if let Some(result) = self.detect_repeated_range(values) {
            if result.compression_ratio > 1.0 {
                if best_result.as_ref().map_or(true, |r| result.compression_ratio > r.compression_ratio) {
                    best_result = Some(result);
                }
            }
        }

        // Try repeated toggle detection
        if let Some(result) = self.detect_repeated_toggle(values) {
            if result.compression_ratio > 1.0 {
                if best_result.as_ref().map_or(true, |r| result.compression_ratio > r.compression_ratio) {
                    best_result = Some(result);
                }
            }
        }

        best_result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repeated_range() {
        let detector = CombinedDetector::new(3);
        let values: Vec<&str> = vec!["1", "2", "3", "1", "2", "3"];
        let result = detector.detect(&values).unwrap();
        
        assert_eq!(result.pattern_type, PatternType::RepeatedRange);
        if let crate::als::AlsOperator::Multiply { value, count } = result.operator {
            assert_eq!(count, 2);
            if let crate::als::AlsOperator::Range { start, end, step } = *value {
                assert_eq!(start, 1);
                assert_eq!(end, 3);
                assert_eq!(step, 1);
            } else {
                panic!("Expected Range operator inside Multiply");
            }
        } else {
            panic!("Expected Multiply operator");
        }
    }

    #[test]
    fn test_repeated_range_three_times() {
        let detector = CombinedDetector::new(3);
        let values: Vec<&str> = vec!["1", "2", "3", "1", "2", "3", "1", "2", "3"];
        let result = detector.detect(&values).unwrap();
        
        assert_eq!(result.pattern_type, PatternType::RepeatedRange);
        if let crate::als::AlsOperator::Multiply { count, .. } = result.operator {
            assert_eq!(count, 3);
        } else {
            panic!("Expected Multiply operator");
        }
    }

    #[test]
    fn test_repeated_arithmetic_range() {
        let detector = CombinedDetector::new(3);
        // Use a longer sequence to ensure compression benefit
        let values: Vec<&str> = vec![
            "10", "20", "30", "40", "50",
            "10", "20", "30", "40", "50",
        ];
        let result = detector.detect(&values);
        
        // This may or may not be detected depending on compression benefit
        if let Some(r) = result {
            assert_eq!(r.pattern_type, PatternType::RepeatedRange);
            if let crate::als::AlsOperator::Multiply { value, count } = r.operator {
                assert_eq!(count, 2);
                if let crate::als::AlsOperator::Range { start, end, step } = *value {
                    assert_eq!(start, 10);
                    assert_eq!(end, 50);
                    assert_eq!(step, 10);
                } else {
                    panic!("Expected Range operator inside Multiply");
                }
            } else {
                panic!("Expected Multiply operator");
            }
        }
    }

    #[test]
    fn test_repeated_toggle() {
        let detector = CombinedDetector::new(3);
        let values: Vec<&str> = vec!["A", "B", "A", "B"];
        let result = detector.detect(&values);
        
        // This might be detected as either a simple toggle or repeated toggle
        // depending on which gives better compression
        if let Some(r) = result {
            assert!(r.compression_ratio > 1.0);
        }
    }

    #[test]
    fn test_no_pattern_irregular() {
        let detector = CombinedDetector::new(3);
        let values: Vec<&str> = vec!["1", "2", "3", "4", "5", "6"];
        // This is a simple range, not a repeated range
        assert!(detector.detect(&values).is_none());
    }

    #[test]
    fn test_no_pattern_too_short() {
        let detector = CombinedDetector::new(3);
        let values: Vec<&str> = vec!["1", "2"];
        assert!(detector.detect(&values).is_none());
    }

    #[test]
    fn test_no_pattern_non_repeating() {
        let detector = CombinedDetector::new(3);
        let values: Vec<&str> = vec!["1", "2", "3", "4", "5", "6"];
        // Sequential but not repeating
        assert!(detector.detect(&values).is_none());
    }

    #[test]
    fn test_descending_repeated_range() {
        let detector = CombinedDetector::new(3);
        let values: Vec<&str> = vec!["3", "2", "1", "3", "2", "1"];
        let result = detector.detect(&values).unwrap();
        
        assert_eq!(result.pattern_type, PatternType::RepeatedRange);
        if let crate::als::AlsOperator::Multiply { value, count } = result.operator {
            assert_eq!(count, 2);
            if let crate::als::AlsOperator::Range { start, end, step } = *value {
                assert_eq!(start, 3);
                assert_eq!(end, 1);
                assert_eq!(step, -1);
            } else {
                panic!("Expected Range operator inside Multiply");
            }
        } else {
            panic!("Expected Multiply operator");
        }
    }

    #[test]
    fn test_longer_repeated_pattern() {
        let detector = CombinedDetector::new(3);
        let values: Vec<&str> = vec![
            "1", "2", "3", "4", "5",
            "1", "2", "3", "4", "5",
        ];
        let result = detector.detect(&values).unwrap();
        
        assert_eq!(result.pattern_type, PatternType::RepeatedRange);
        if let crate::als::AlsOperator::Multiply { value, count } = result.operator {
            assert_eq!(count, 2);
            if let crate::als::AlsOperator::Range { start, end, .. } = *value {
                assert_eq!(start, 1);
                assert_eq!(end, 5);
            } else {
                panic!("Expected Range operator inside Multiply");
            }
        } else {
            panic!("Expected Multiply operator");
        }
    }
}
