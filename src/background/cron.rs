//! Cron expression parser and evaluator
//!
//! This module provides a cron expression parser that supports standard cron syntax:
//! - Minute (0-59)
//! - Hour (0-23)
//! - Day of month (1-31)
//! - Month (1-12)
//! - Day of week (0-6, Sunday=0)
//!
//! Supported operators:
//! - `*` - Any value
//! - `,` - Value list separator
//! - `-` - Range of values
//! - `/` - Step values
//!
//! Examples:
//! - `* * * * *` - Every minute
//! - `0 * * * *` - Every hour
//! - `0 0 * * *` - Every day at midnight
//! - `*/15 * * * *` - Every 15 minutes
//! - `0 9-17 * * 1-5` - Every hour from 9 AM to 5 PM on weekdays

use chrono::{DateTime, Datelike, Local, Timelike};
use std::collections::HashSet;

/// Cron expression parsing error
#[derive(Debug, Clone, PartialEq)]
pub enum CronError {
    /// Invalid format
    InvalidFormat(String),
    /// Invalid field value
    InvalidValue(String),
    /// Invalid range
    InvalidRange(String),
}

impl std::fmt::Display for CronError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CronError::InvalidFormat(msg) => write!(f, "Invalid cron format: {}", msg),
            CronError::InvalidValue(msg) => write!(f, "Invalid value: {}", msg),
            CronError::InvalidRange(msg) => write!(f, "Invalid range: {}", msg),
        }
    }
}

impl std::error::Error for CronError {}

/// Cron field type
#[derive(Debug, Clone)]
enum CronField {
    /// Any value (*)
    Any,
    /// Specific values
    Values(HashSet<u32>),
    /// Range with optional step
    Range { start: u32, end: u32, step: u32 },
}

impl CronField {
    /// Parse a cron field
    fn parse(s: &str, min: u32, max: u32) -> Result<Self, CronError> {
        if s == "*" {
            return Ok(CronField::Any);
        }

        // Handle step values (e.g., */15)
        if s.contains('/') {
            let parts: Vec<&str> = s.split('/').collect();
            if parts.len() != 2 {
                return Err(CronError::InvalidFormat(format!(
                    "Invalid step syntax: {}",
                    s
                )));
            }

            let step = parts[1]
                .parse::<u32>()
                .map_err(|_| CronError::InvalidValue(format!("Invalid step: {}", parts[1])))?;

            if parts[0] == "*" {
                return Ok(CronField::Range {
                    start: min,
                    end: max,
                    step,
                });
            } else if parts[0].contains('-') {
                let range_parts: Vec<&str> = parts[0].split('-').collect();
                if range_parts.len() != 2 {
                    return Err(CronError::InvalidFormat(format!("Invalid range: {}", parts[0])));
                }
                let start = range_parts[0].parse::<u32>().map_err(|_| {
                    CronError::InvalidValue(format!("Invalid start: {}", range_parts[0]))
                })?;
                let end = range_parts[1].parse::<u32>().map_err(|_| {
                    CronError::InvalidValue(format!("Invalid end: {}", range_parts[1]))
                })?;
                return Ok(CronField::Range { start, end, step });
            }
        }

        // Handle ranges (e.g., 1-5)
        if s.contains('-') {
            let parts: Vec<&str> = s.split('-').collect();
            if parts.len() != 2 {
                return Err(CronError::InvalidFormat(format!("Invalid range: {}", s)));
            }

            let start = parts[0]
                .parse::<u32>()
                .map_err(|_| CronError::InvalidValue(format!("Invalid start: {}", parts[0])))?;
            let end = parts[1]
                .parse::<u32>()
                .map_err(|_| CronError::InvalidValue(format!("Invalid end: {}", parts[1])))?;

            if start < min || start > max || end < min || end > max || start > end {
                return Err(CronError::InvalidRange(format!(
                    "Range {}-{} outside allowed range {}-{}",
                    start, end, min, max
                )));
            }

            return Ok(CronField::Range {
                start,
                end,
                step: 1,
            });
        }

        // Handle comma-separated values (e.g., 1,3,5)
        if s.contains(',') {
            let parts: Vec<&str> = s.split(',').collect();
            let mut values = HashSet::new();

            for part in parts {
                let value = part
                    .parse::<u32>()
                    .map_err(|_| CronError::InvalidValue(format!("Invalid value: {}", part)))?;
                if value < min || value > max {
                    return Err(CronError::InvalidValue(format!(
                        "Value {} outside allowed range {}-{}",
                        value, min, max
                    )));
                }
                values.insert(value);
            }

            return Ok(CronField::Values(values));
        }

        // Single value
        let value = s
            .parse::<u32>()
            .map_err(|_| CronError::InvalidValue(format!("Invalid value: {}", s)))?;

        if value < min || value > max {
            return Err(CronError::InvalidValue(format!(
                "Value {} outside allowed range {}-{}",
                value, min, max
            )));
        }

        let mut values = HashSet::new();
        values.insert(value);
        Ok(CronField::Values(values))
    }

    /// Check if a value matches this field
    fn matches(&self, value: u32) -> bool {
        match self {
            CronField::Any => true,
            CronField::Values(values) => values.contains(&value),
            CronField::Range { start, end, step } => {
                value >= *start && value <= *end && (value - start) % step == 0
            }
        }
    }
}

/// Cron expression
#[derive(Debug, Clone)]
pub struct CronExpr {
    minute: CronField,
    hour: CronField,
    day: CronField,
    month: CronField,
    weekday: CronField,
}

impl CronExpr {
    /// Parse a cron expression
    ///
    /// Format: minute hour day month weekday
    /// - minute: 0-59
    /// - hour: 0-23
    /// - day: 1-31
    /// - month: 1-12
    /// - weekday: 0-6 (Sunday=0)
    pub fn parse(expr: &str) -> Result<Self, CronError> {
        let parts: Vec<&str> = expr.split_whitespace().collect();

        if parts.len() != 5 {
            return Err(CronError::InvalidFormat(format!(
                "Expected 5 fields, got {}",
                parts.len()
            )));
        }

        Ok(Self {
            minute: CronField::parse(parts[0], 0, 59)?,
            hour: CronField::parse(parts[1], 0, 23)?,
            day: CronField::parse(parts[2], 1, 31)?,
            month: CronField::parse(parts[3], 1, 12)?,
            weekday: CronField::parse(parts[4], 0, 6)?,
        })
    }

    /// Check if the expression matches a given datetime
    pub fn matches(&self, dt: &DateTime<Local>) -> bool {
        self.minute.matches(dt.minute()) &&
        self.hour.matches(dt.hour()) &&
        self.day.matches(dt.day()) &&
        self.month.matches(dt.month()) &&
        self.weekday.matches(dt.weekday().num_days_from_sunday())
    }

    /// Find the next execution time after the given datetime
    pub fn next(&self, after: &DateTime<Local>) -> Option<DateTime<Local>> {
        // Simple implementation: check every minute for the next year
        let mut current = after.clone();
        let max_checks = 365 * 24 * 60; // One year worth of minutes

        for _ in 0..max_checks {
            current = current + chrono::Duration::minutes(1);
            if self.matches(&current) {
                return Some(current);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_expressions() {
        // Every minute
        let expr = CronExpr::parse("* * * * *").unwrap();
        let now = Local::now();
        assert!(expr.matches(&now));

        // Specific minute
        let expr = CronExpr::parse("30 * * * *").unwrap();
        // Should match when minute is 30

        // Every hour at minute 0
        let expr = CronExpr::parse("0 * * * *").unwrap();
        // Should match when minute is 0
    }

    #[test]
    fn test_parse_ranges() {
        // Weekdays (Monday-Friday)
        let expr = CronExpr::parse("0 9 * * 1-5").unwrap();

        // Hours range
        let expr = CronExpr::parse("0 9-17 * * *").unwrap();
    }

    #[test]
    fn test_parse_step_values() {
        // Every 15 minutes
        let expr = CronExpr::parse("*/15 * * * *").unwrap();

        // Every 2 hours
        let expr = CronExpr::parse("0 */2 * * *").unwrap();
    }

    #[test]
    fn test_parse_comma_separated() {
        // Specific minutes
        let expr = CronExpr::parse("0,15,30,45 * * * *").unwrap();

        // Specific hours
        let expr = CronExpr::parse("0 9,12,15,18 * * *").unwrap();
    }

    #[test]
    fn test_parse_invalid_expressions() {
        // Too few fields
        assert!(CronExpr::parse("* * *").is_err());

        // Too many fields
        assert!(CronExpr::parse("* * * * * *").is_err());

        // Invalid value
        assert!(CronExpr::parse("60 * * * *").is_err());

        // Invalid range
        assert!(CronExpr::parse("5-3 * * * *").is_err());
    }

    #[test]
    fn test_field_matching() {
        let field = CronField::parse("*/15", 0, 59).unwrap();
        assert!(field.matches(0));
        assert!(field.matches(15));
        assert!(field.matches(30));
        assert!(field.matches(45));
        assert!(!field.matches(10));
        assert!(!field.matches(20));

        let field = CronField::parse("1-5", 0, 59).unwrap();
        assert!(field.matches(1));
        assert!(field.matches(3));
        assert!(field.matches(5));
        assert!(!field.matches(0));
        assert!(!field.matches(6));

        let field = CronField::parse("1,3,5", 0, 59).unwrap();
        assert!(field.matches(1));
        assert!(field.matches(3));
        assert!(field.matches(5));
        assert!(!field.matches(2));
        assert!(!field.matches(4));
    }

    #[test]
    fn test_cron_matching() {
        // Create a specific datetime
        use chrono::TimeZone;

        // Monday, 2024-01-01 09:30:00
        let dt = Local.with_ymd_and_hms(2024, 1, 1, 9, 30, 0).unwrap();

        // Should match: every minute
        let expr = CronExpr::parse("* * * * *").unwrap();
        assert!(expr.matches(&dt));

        // Should match: minute 30
        let expr = CronExpr::parse("30 * * * *").unwrap();
        assert!(expr.matches(&dt));

        // Should not match: minute 0
        let expr = CronExpr::parse("0 * * * *").unwrap();
        assert!(!expr.matches(&dt));

        // Should match: hour 9
        let expr = CronExpr::parse("30 9 * * *").unwrap();
        assert!(expr.matches(&dt));

        // Should match: January (month 1)
        let expr = CronExpr::parse("30 9 1 1 *").unwrap();
        assert!(expr.matches(&dt));
    }

    #[test]
    fn test_next_execution() {
        use chrono::TimeZone;

        // Current time: 09:00
        let now = Local.with_ymd_and_hms(2024, 1, 1, 9, 0, 0).unwrap();

        // Next execution at 09:30
        let expr = CronExpr::parse("30 * * * *").unwrap();
        let next = expr.next(&now).unwrap();
        assert_eq!(next.hour(), 9);
        assert_eq!(next.minute(), 30);

        // Next execution at 10:00
        let expr = CronExpr::parse("0 10 * * *").unwrap();
        let next = expr.next(&now).unwrap();
        assert_eq!(next.hour(), 10);
        assert_eq!(next.minute(), 0);
    }
}
