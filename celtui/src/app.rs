//! Application state and event handling
//!
//! This module defines the core application structure, state management,
//! and screen navigation system for the Celestial Navigation TUI.

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use std::time::Duration;
use crate::calculation_screen::CalculationForm;
use crate::almanac_screen::AlmanacForm;
use crate::sight_reduction_screen::SightReductionForm;
use crate::auto_compute_screen::AutoComputeForm;
use crate::averaging_screen::AveragingForm;
use crate::arc_to_time_screen::ArcToTimeForm;

/// Represents the different screens/views in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Main menu / home screen
    Home,
    /// Almanac data lookup screen
    Almanac,
    /// Sight reduction tables screen
    SightReduction,
    /// Automatic computation screen
    AutoCompute,
    /// Calculation screen (comprehensive sight reduction form)
    Calculation,
    /// Sight averaging screen
    Averaging,
    /// Arc to Time calculator screen
    ArcToTime,
    /// Help / instructions screen
    Help,
}

/// Main application state
pub struct App {
    /// Current active screen
    pub current_screen: Screen,
    /// Whether the application should quit
    pub should_quit: bool,
    /// Calculation form state
    pub calculation_form: CalculationForm,
    /// Almanac form state
    pub almanac_form: AlmanacForm,
    /// Sight reduction form state
    pub sight_reduction_form: SightReductionForm,
    /// Auto compute form state
    pub auto_compute_form: AutoComputeForm,
    /// Averaging form state
    pub averaging_form: AveragingForm,
    /// Arc to Time form state
    pub arc_to_time_form: ArcToTimeForm,
}

impl App {
    /// Creates a new application instance
    pub fn new() -> Self {
        Self {
            current_screen: Screen::Home,
            should_quit: false,
            calculation_form: CalculationForm::new(),
            almanac_form: AlmanacForm::new(),
            sight_reduction_form: SightReductionForm::new(),
            auto_compute_form: AutoComputeForm::new(),
            averaging_form: AveragingForm::new(),
            arc_to_time_form: ArcToTimeForm::new(),
        }
    }

    /// Handles keyboard events and updates application state
    ///
    /// # Arguments
    /// * `key_event` - The keyboard event to handle
    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        // Only handle key press events (not release or repeat)
        if key_event.kind != KeyEventKind::Press {
            return;
        }

        // If on calculation screen, let it handle most keys first
        if self.current_screen == Screen::Calculation {
            // Check if we're in text input mode
            let text_input_active = self.calculation_form.is_text_input_active();

            match key_event.code {
                // Always allow emergency exits (q/Q and Esc) to navigate away
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    // Fall through to normal navigation
                }
                // Only allow screen navigation shortcuts if NOT in text input mode
                KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?')
                | KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Char('s') | KeyCode::Char('S')
                | KeyCode::Char('c') | KeyCode::Char('C') if !text_input_active => {
                    // Fall through to normal navigation
                }
                // All other keys are handled by the calculation form
                _ => {
                    self.calculation_form.handle_key_event(key_event);
                    return;
                }
            }
        }

        // If on almanac screen, let it handle most keys first
        if self.current_screen == Screen::Almanac {
            // Check if we're in text input mode
            let text_input_active = self.almanac_form.is_text_input_active();

            match key_event.code {
                // Always allow emergency exits (q/Q and Esc) to navigate away
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    // Fall through to normal navigation
                }
                // Only allow screen navigation shortcuts if NOT in text input mode
                KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?')
                | KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Char('s') | KeyCode::Char('S')
                | KeyCode::Char('c') | KeyCode::Char('C') if !text_input_active => {
                    // Fall through to normal navigation
                }
                // All other keys are handled by the almanac form
                _ => {
                    self.almanac_form.handle_key_event(key_event);
                    return;
                }
            }
        }

        // If on sight reduction screen, let it handle most keys first
        if self.current_screen == Screen::SightReduction {
            match key_event.code {
                // Always allow emergency exits (q/Q and Esc) to navigate away
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    // Fall through to normal navigation
                }
                // Allow other navigation shortcuts (sight reduction has no text input)
                KeyCode::Char('h') | KeyCode::Char('H')
                | KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Char('s') | KeyCode::Char('S')
                | KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Char('?') => {
                    // Fall through to normal navigation
                }
                // All other keys are handled by the sight reduction form
                _ => {
                    self.sight_reduction_form.handle_key_event(key_event);
                    return;
                }
            }
        }

        // If on auto compute screen, let it handle most keys first
        if self.current_screen == Screen::AutoCompute {
            // Check if we're in text input mode
            let text_input_active = self.auto_compute_form.is_text_input_active();

            match key_event.code {
                // Always allow emergency exits (q/Q and Esc) to navigate away
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    // Fall through to normal navigation
                }
                // Only allow screen navigation shortcuts if NOT in text input mode
                // Note: 'c' is not included here since it's used for "compute fix" in auto compute
                KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?')
                | KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Char('s') | KeyCode::Char('S') if !text_input_active => {
                    // Fall through to normal navigation
                }
                // All other keys are handled by the auto compute form
                _ => {
                    self.auto_compute_form.handle_key_event(key_event);
                    return;
                }
            }
        }

        // If on averaging screen, let it handle most keys first
        if self.current_screen == Screen::Averaging {
            match key_event.code {
                // Always allow emergency exits (q/Q and Esc) to navigate away
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    // Fall through to normal navigation
                }
                // Allow other navigation shortcuts (averaging has no text input)
                KeyCode::Char('h') | KeyCode::Char('H')
                | KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Char('s') | KeyCode::Char('S')
                | KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Char('?') | KeyCode::Char('v') | KeyCode::Char('V') => {
                    // Fall through to normal navigation
                }
                // All other keys are handled by the averaging form
                _ => {
                    self.averaging_form.handle_key_event(key_event);
                    return;
                }
            }
        }

        // If on arc to time screen, let it handle most keys first
        if self.current_screen == Screen::ArcToTime {
            match key_event.code {
                // Always allow emergency exits (q/Q and Esc) to navigate away
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                    // Fall through to normal navigation
                }
                // Allow other navigation shortcuts (arc to time has no text input)
                KeyCode::Char('h') | KeyCode::Char('H')
                | KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Char('s') | KeyCode::Char('S')
                | KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Char('?') | KeyCode::Char('v') | KeyCode::Char('V') => {
                    // Fall through to normal navigation
                }
                // All other keys are handled by the arc to time form
                _ => {
                    self.arc_to_time_form.handle_key_event(key_event);
                    return;
                }
            }
        }

        match key_event.code {
            // Global keybindings
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                if self.current_screen == Screen::Home {
                    self.should_quit = true;
                } else {
                    // Return to home screen if not already there
                    self.current_screen = Screen::Home;
                }
            }
            KeyCode::Esc => {
                // Escape always returns to home
                self.current_screen = Screen::Home;
            }

            // Navigation keybindings
            KeyCode::Char('h') | KeyCode::Char('H') => {
                self.current_screen = Screen::Home;
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.current_screen = Screen::Almanac;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                self.current_screen = Screen::SightReduction;
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                self.current_screen = Screen::Calculation;
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                self.current_screen = Screen::Averaging;
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.current_screen = Screen::ArcToTime;
            }
            KeyCode::Char('?') => {
                self.current_screen = Screen::Help;
            }

            // Number key navigation (from home screen)
            KeyCode::Char('1') if self.current_screen == Screen::Home => {
                self.current_screen = Screen::Almanac;
            }
            KeyCode::Char('2') if self.current_screen == Screen::Home => {
                self.current_screen = Screen::SightReduction;
            }
            KeyCode::Char('3') if self.current_screen == Screen::Home => {
                self.current_screen = Screen::Calculation;
            }
            KeyCode::Char('4') if self.current_screen == Screen::Home => {
                self.current_screen = Screen::AutoCompute;
            }
            KeyCode::Char('5') if self.current_screen == Screen::Home => {
                self.current_screen = Screen::Averaging;
            }
            KeyCode::Char('6') if self.current_screen == Screen::Home => {
                self.current_screen = Screen::ArcToTime;
            }

            // Tab to cycle through screens
            KeyCode::Tab => {
                self.current_screen = match self.current_screen {
                    Screen::Home => Screen::Almanac,
                    Screen::Almanac => Screen::SightReduction,
                    Screen::SightReduction => Screen::AutoCompute,
                    Screen::AutoCompute => Screen::Calculation,
                    Screen::Calculation => Screen::Averaging,
                    Screen::Averaging => Screen::ArcToTime,
                    Screen::ArcToTime => Screen::Help,
                    Screen::Help => Screen::Home,
                };
            }

            // Backtab (Shift+Tab) to cycle backwards
            KeyCode::BackTab => {
                self.current_screen = match self.current_screen {
                    Screen::Home => Screen::Help,
                    Screen::Almanac => Screen::Home,
                    Screen::SightReduction => Screen::Almanac,
                    Screen::AutoCompute => Screen::SightReduction,
                    Screen::Calculation => Screen::AutoCompute,
                    Screen::Averaging => Screen::Calculation,
                    Screen::ArcToTime => Screen::Averaging,
                    Screen::Help => Screen::ArcToTime,
                };
            }

            _ => {}
        }
    }

    /// Handles all events (keyboard, mouse, resize, etc.)
    ///
    /// # Arguments
    /// * `timeout` - Maximum time to wait for an event
    ///
    /// # Returns
    /// `Ok(true)` if an event was handled, `Ok(false)` if no event occurred
    pub fn handle_events(&mut self, timeout: Duration) -> Result<bool> {
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key_event) => {
                    self.handle_key_event(key_event);
                    Ok(true)
                }
                Event::Resize(_, _) => {
                    // Terminal was resized, trigger redraw
                    Ok(true)
                }
                _ => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    /// Returns the title for the current screen
    pub fn current_screen_title(&self) -> &str {
        match self.current_screen {
            Screen::Home => "Celestial Navigation TUI - Home",
            Screen::Almanac => "Almanac Data Lookup",
            Screen::SightReduction => "Sight Reduction Tables",
            Screen::AutoCompute => "Automatic Fix Computation",
            Screen::Calculation => "Sight Reduction Calculator",
            Screen::Averaging => "Sight Averaging",
            Screen::ArcToTime => "Arc to Time Calculator",
            Screen::Help => "Help & Instructions",
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = App::new();
        assert_eq!(app.current_screen, Screen::Home);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_quit_from_home() {
        let mut app = App::new();
        app.handle_key_event(KeyEvent::from(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn test_quit_from_other_screen_returns_home() {
        let mut app = App::new();
        app.current_screen = Screen::Almanac;
        app.handle_key_event(KeyEvent::from(KeyCode::Char('q')));
        assert_eq!(app.current_screen, Screen::Home);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_escape_returns_home() {
        let mut app = App::new();
        app.current_screen = Screen::SightReduction;
        app.handle_key_event(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.current_screen, Screen::Home);
    }

    #[test]
    fn test_tab_navigation_from_home() {
        let mut app = App::new();
        assert_eq!(app.current_screen, Screen::Home);

        // Tab works for screen navigation from home
        app.handle_key_event(KeyEvent::from(KeyCode::Tab));
        assert_eq!(app.current_screen, Screen::Almanac);

        // Once in a form screen with text input active, letter keys type into fields
        // Use Esc to exit back to home, then navigate
        app.handle_key_event(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.current_screen, Screen::Home);

        app.handle_key_event(KeyEvent::from(KeyCode::Char('s')));
        assert_eq!(app.current_screen, Screen::SightReduction);
    }

    #[test]
    fn test_letter_navigation() {
        let mut app = App::new();

        app.handle_key_event(KeyEvent::from(KeyCode::Char('a')));
        assert_eq!(app.current_screen, Screen::Almanac);

        // When on Almanac screen, default field is Date (text input), so 's' won't navigate
        // Either use Esc to return home first, or use a screen that's not in text mode
        app.handle_key_event(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.current_screen, Screen::Home);

        app.handle_key_event(KeyEvent::from(KeyCode::Char('s')));
        assert_eq!(app.current_screen, Screen::SightReduction);

        app.handle_key_event(KeyEvent::from(KeyCode::Char('h')));
        assert_eq!(app.current_screen, Screen::Home);
    }

    #[test]
    fn test_number_navigation_from_home() {
        let mut app = App::new();

        app.handle_key_event(KeyEvent::from(KeyCode::Char('1')));
        assert_eq!(app.current_screen, Screen::Almanac);

        app.current_screen = Screen::Home;
        app.handle_key_event(KeyEvent::from(KeyCode::Char('2')));
        assert_eq!(app.current_screen, Screen::SightReduction);
    }

    // Phase 2: Text input mode tests

    #[test]
    fn test_screen_shortcuts_disabled_during_text_input_calculation() {
        let mut app = App::new();
        app.current_screen = Screen::Calculation;
        app.calculation_form.current_field = crate::calculation_screen::InputField::StarName;
        app.calculation_form.celestial_body = crate::calculation_screen::CelestialBody::Star;

        // When typing in StarName, pressing 'a' should add 'a' to the field, not switch to Almanac screen
        let initial_screen = app.current_screen;
        app.handle_key_event(KeyEvent::from(KeyCode::Char('a')));
        assert_eq!(app.current_screen, initial_screen, "Should not switch screens when typing in text field");
        // Note: The actual character input is handled by calculation_form.handle_key_event
    }

    #[test]
    fn test_screen_shortcuts_work_when_not_in_text_input() {
        let mut app = App::new();
        app.current_screen = Screen::Calculation;
        app.calculation_form.current_field = crate::calculation_screen::InputField::CelestialBody;

        // When on CelestialBody field (not text input), 'a' should switch to Almanac
        app.handle_key_event(KeyEvent::from(KeyCode::Char('a')));
        assert_eq!(app.current_screen, Screen::Almanac, "Should switch to Almanac when not in text input field");
    }

    #[test]
    fn test_almanac_screen_shortcuts_disabled_during_text_input() {
        let mut app = App::new();
        app.current_screen = Screen::Almanac;
        app.almanac_form.current_field = crate::almanac_screen::AlmanacInputField::StarName;

        // When typing in StarName, pressing 'c' should add 'c' to the field
        let initial_screen = app.current_screen;
        app.handle_key_event(KeyEvent::from(KeyCode::Char('c')));
        assert_eq!(app.current_screen, initial_screen, "Should not switch screens when typing in text field");
    }

    #[test]
    fn test_auto_compute_screen_shortcuts_disabled_during_text_input() {
        let mut app = App::new();
        app.current_screen = Screen::AutoCompute;
        app.auto_compute_form.mode = crate::auto_compute_screen::AutoComputeMode::EnteringSight;
        app.auto_compute_form.current_field = crate::auto_compute_screen::SightInputField::Date;

        // When typing in Date field, pressing 'a' should add 'a' to the field
        let initial_screen = app.current_screen;
        app.handle_key_event(KeyEvent::from(KeyCode::Char('a')));
        assert_eq!(app.current_screen, initial_screen, "Should not switch screens when typing in text field");
    }

    #[test]
    fn test_comprehensive_user_journey_typing_star_name() {
        // This test simulates a user typing "Altair" in the star name field
        // and verifies that the letters don't trigger screen shortcuts
        let mut app = App::new();

        // Navigate to Calculation screen
        app.current_screen = Screen::Calculation;
        app.calculation_form.current_field = crate::calculation_screen::InputField::StarName;
        app.calculation_form.celestial_body = crate::calculation_screen::CelestialBody::Star;

        // Type "Altair" - each letter should be handled by the form, not trigger shortcuts
        let star_name = "Altair";
        for ch in star_name.chars() {
            let initial_screen = app.current_screen;
            app.handle_key_event(KeyEvent::from(KeyCode::Char(ch)));
            assert_eq!(
                app.current_screen, initial_screen,
                "Typing '{}' should not change screen from Calculation", ch
            );
        }

        // Verify the star name was typed (note: actual character addition is in calculation_form)
        // The important thing is we stayed on the Calculation screen
        assert_eq!(app.current_screen, Screen::Calculation);

        // Now if we switch to a non-text field, shortcuts should work again
        app.calculation_form.current_field = crate::calculation_screen::InputField::CelestialBody;
        app.handle_key_event(KeyEvent::from(KeyCode::Char('a')));
        assert_eq!(app.current_screen, Screen::Almanac, "Should switch to Almanac when not in text input");
    }

    // TDD tests for Issue 2: "H" key should type in text fields, not trigger Home screen

    #[test]
    fn test_h_key_types_in_star_name_field() {
        let mut app = App::new();
        app.current_screen = Screen::Calculation;
        app.calculation_form.current_field = crate::calculation_screen::InputField::StarName;
        app.calculation_form.celestial_body = crate::calculation_screen::CelestialBody::Star;

        // Typing "H" should stay on Calculation screen (not go to Home)
        app.handle_key_event(KeyEvent::from(KeyCode::Char('H')));
        assert_eq!(app.current_screen, Screen::Calculation,
            "Pressing 'H' in text field should not navigate to Home screen");

        // Lowercase 'h' should also stay on Calculation screen
        app.handle_key_event(KeyEvent::from(KeyCode::Char('h')));
        assert_eq!(app.current_screen, Screen::Calculation,
            "Pressing 'h' in text field should not navigate to Home screen");
    }

    #[test]
    fn test_h_key_types_when_entering_hadar_star_name() {
        let mut app = App::new();
        app.current_screen = Screen::Calculation;
        app.calculation_form.current_field = crate::calculation_screen::InputField::StarName;
        app.calculation_form.celestial_body = crate::calculation_screen::CelestialBody::Star;

        // Simulate typing "Hadar" - the 'H' should not trigger Home screen
        for ch in "Hadar".chars() {
            app.handle_key_event(KeyEvent::from(KeyCode::Char(ch)));
            assert_eq!(app.current_screen, Screen::Calculation,
                "Typing '{}' should not navigate away from Calculation screen", ch);
        }
    }

    #[test]
    fn test_help_key_still_works_when_not_in_text_input() {
        let mut app = App::new();
        app.current_screen = Screen::Calculation;
        app.calculation_form.current_field = crate::calculation_screen::InputField::CelestialBody;

        // When NOT in text input, '?' should still open help
        app.handle_key_event(KeyEvent::from(KeyCode::Char('?')));
        assert_eq!(app.current_screen, Screen::Help,
            "? key should open Help screen when not in text input mode");
    }

    #[test]
    fn test_q_and_esc_always_work_even_in_text_input() {
        let mut app = App::new();
        app.current_screen = Screen::Calculation;
        app.calculation_form.current_field = crate::calculation_screen::InputField::StarName;

        // 'q' should still return to Home (emergency exit)
        app.handle_key_event(KeyEvent::from(KeyCode::Char('q')));
        assert_eq!(app.current_screen, Screen::Home,
            "'q' should always work as emergency exit even in text input");

        // Navigate back to Calculation
        app.current_screen = Screen::Calculation;
        app.calculation_form.current_field = crate::calculation_screen::InputField::StarName;

        // Esc should still return to Home
        app.handle_key_event(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.current_screen, Screen::Home,
            "Esc should always work as emergency exit even in text input");
    }
}
