//! Animated ASCII progress bar generation

use std::time::Duration;

/// Animated progress bar for terminal display
pub struct ProgressBar {
    width: usize,
    filled_char: char,
    empty_char: char,
}

impl ProgressBar {
    /// Create a new progress bar
    pub fn new(width: usize) -> Self {
        ProgressBar {
            width,
            filled_char: '█',
            empty_char: '░',
        }
    }

    /// Create progress bar with custom characters
    pub fn with_chars(width: usize, filled: char, empty: char) -> Self {
        ProgressBar {
            width,
            filled_char: filled,
            empty_char: empty,
        }
    }

    /// Render bar for a given percentage
    pub fn render(&self, percent: f64) -> String {
        let percent = percent.max(0.0).min(100.0);
        let filled = ((percent / 100.0) * self.width as f64) as usize;
        let empty = self.width - filled;

        format!(
            "{}{}",
            self.filled_char.to_string().repeat(filled),
            self.empty_char.to_string().repeat(empty)
        )
    }

    /// Render bar for a duration relative to total
    pub fn render_duration(&self, duration: &Duration, total: &Duration) -> String {
        if total.as_millis() == 0 {
            return self.render(0.0);
        }

        let percent = (duration.as_millis() as f64 / total.as_millis() as f64) * 100.0;
        self.render(percent)
    }

    /// Create an animated spinning cursor (4 frames)
    pub fn spinner(frame: usize) -> &'static str {
        match frame % 4 {
            0 => "⠋",
            1 => "⠙",
            2 => "⠹",
            _ => "⠸",
        }
    }

    /// Create a pulsing animation effect (3 frames)
    pub fn pulse(frame: usize) -> &'static str {
        match frame % 3 {
            0 => "◐",
            1 => "◓",
            _ => "◑",
        }
    }

    /// Create a bouncing animation effect
    pub fn bouncer(frame: usize, width: usize) -> String {
        let position = (frame % (width * 2)) as usize;
        let pos = if position >= width {
            width * 2 - position - 1
        } else {
            position
        };

        let mut bar = String::new();
        for i in 0..width {
            if i == pos {
                bar.push('●');
            } else {
                bar.push('○');
            }
        }
        bar
    }

    /// Format phase output with animated bar
    pub fn format_phase(
        phase_name: &str,
        duration: &Duration,
        total: &Duration,
        color: &str,
        reset: &str,
    ) -> String {
        let bar = ProgressBar::new(15).render_duration(duration, total);
        let percent = if total.as_millis() == 0 {
            0.0
        } else {
            (duration.as_millis() as f64 / total.as_millis() as f64) * 100.0
        };

        format!(
            "  {:<25} {}{}{}  {:>6}ms  {:>5.1}%{}",
            phase_name,
            color,
            bar,
            reset,
            duration.as_millis(),
            percent,
            reset
        )
    }
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self::new(15)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_creation() {
        let bar = ProgressBar::new(10);
        assert_eq!(bar.width, 10);
    }

    #[test]
    fn test_progress_bar_render_zero() {
        let bar = ProgressBar::new(10);
        let rendered = bar.render(0.0);
        assert_eq!(rendered, "░░░░░░░░░░");
    }

    #[test]
    fn test_progress_bar_render_full() {
        let bar = ProgressBar::new(10);
        let rendered = bar.render(100.0);
        assert_eq!(rendered, "██████████");
    }

    #[test]
    fn test_progress_bar_render_half() {
        let bar = ProgressBar::new(10);
        let rendered = bar.render(50.0);
        assert_eq!(rendered, "█████░░░░░");
    }

    #[test]
    fn test_progress_bar_custom_chars() {
        let bar = ProgressBar::with_chars(10, '=', '-');
        let rendered = bar.render(50.0);
        assert_eq!(rendered, "=====-----");
    }

    #[test]
    fn test_progress_bar_clamp() {
        let bar = ProgressBar::new(10);
        let over = bar.render(150.0);
        assert_eq!(over, "██████████");

        let under = bar.render(-50.0);
        assert_eq!(under, "░░░░░░░░░░");
    }

    #[test]
    fn test_progress_bar_duration() {
        let bar = ProgressBar::new(10);
        let duration = Duration::from_millis(5);
        let total = Duration::from_millis(10);
        let rendered = bar.render_duration(&duration, &total);
        assert_eq!(rendered, "█████░░░░░");
    }

    #[test]
    fn test_spinner_animation() {
        assert_eq!(ProgressBar::spinner(0), "⠋");
        assert_eq!(ProgressBar::spinner(1), "⠙");
        assert_eq!(ProgressBar::spinner(2), "⠹");
        assert_eq!(ProgressBar::spinner(3), "⠸");
        assert_eq!(ProgressBar::spinner(4), "⠋"); // Loops
    }

    #[test]
    fn test_pulse_animation() {
        assert_eq!(ProgressBar::pulse(0), "◐");
        assert_eq!(ProgressBar::pulse(1), "◓");
        assert_eq!(ProgressBar::pulse(2), "◑");
        assert_eq!(ProgressBar::pulse(3), "◐"); // Loops
    }

    #[test]
    fn test_bouncer_animation() {
        let bounce = ProgressBar::bouncer(0, 5);
        assert!(bounce.contains('●'));
        assert!(bounce.contains('○'));
    }

    #[test]
    fn test_format_phase() {
        let duration = Duration::from_millis(100);
        let total = Duration::from_millis(1000);
        let formatted = ProgressBar::format_phase("Parsing", &duration, &total, "", "");
        assert!(formatted.contains("Parsing"));
        assert!(formatted.contains("100ms"));
        assert!(formatted.contains("10.0%"));
    }
}
