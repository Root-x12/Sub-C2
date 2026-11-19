//! tachyonfx effect engine for OXIDE C2.
//! Manages persistent and transient shader effects that run as post-processing
//! on the ratatui buffer each frame.

use ratatui::layout::Rect;
use ratatui::style::Color;
use std::time::Instant;
use tachyonfx::{fx, Effect, EffectRenderer, Interpolation, Shader};
use tachyonfx::Duration as FxDuration;

use super::{C_BG, C_ACCENT};

/// Manages all active visual effects for the TUI.
pub struct FxEngine {
    /// Persistent glitch on the header branding
    pub header_glitch: Effect,
    /// Tab content area sweep-in on tab switch
    pub tab_transition: Option<Effect>,
    /// Modal open dissolve/coalesce
    pub modal_fx: Option<Effect>,
    /// Pulsing HSL shift on the status bar accent elements
    pub statusbar_pulse: Effect,
    /// Tracks which tab was active last frame to detect switches
    pub last_tab_idx: usize,
    /// Whether a modal was open last frame
    pub had_modal: bool,
    /// Time of last frame for delta calculation
    pub last_frame: Instant,
    /// Startup sweep effect for entire screen
    pub startup_fx: Option<Effect>,
    /// Content area ambient flicker — subtle dissolve cycle
    pub ambient_flicker: Effect,
}

impl FxEngine {
    pub fn new() -> Self {
        Self {
            header_glitch: Self::make_header_glitch(),
            tab_transition: None,
            modal_fx: None,
            statusbar_pulse: Self::make_statusbar_pulse(),
            last_tab_idx: 1, // Agents is default tab
            had_modal: false,
            last_frame: Instant::now(),
            startup_fx: Some(Self::make_startup_effect()),
            ambient_flicker: Self::make_ambient_flicker(),
        }
    }

    /// Calculate delta time since last frame
    pub fn delta(&mut self) -> FxDuration {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame);
        self.last_frame = now;
        FxDuration::from_millis(elapsed.as_millis() as u32)
    }

    /// Persistent glitch effect on the header — subtle character corruption
    fn make_header_glitch() -> Effect {
        fx::never_complete(
            fx::repeating(
                fx::Glitch::builder()
                    .cell_glitch_ratio(0.03)
                    .action_start_delay_ms(200..800)
                    .action_ms(80..250)
                    .build()
                    .into_effect()
            )
        )
    }

    /// Pulsing HSL hue shift on status bar — cycles accent color
    fn make_statusbar_pulse() -> Effect {
        fx::never_complete(
            fx::repeating(
                fx::ping_pong(
                    fx::hsl_shift_fg(
                        [30.0, 0.0, 5.0],
                        (2000, Interpolation::SineInOut),
                    )
                )
            )
        )
    }

    /// Startup effect — sweep in from darkness
    fn make_startup_effect() -> Effect {
        fx::sweep_in(
            fx::Direction::LeftToRight,
            20,     // gradient length
            8,      // randomness
            Color::Rgb(0, 0, 0),
            (800, Interpolation::CubicOut),
        )
    }

    /// Ambient flicker — subtle HSL lightness oscillation on content area
    fn make_ambient_flicker() -> Effect {
        fx::never_complete(
            fx::repeating(
                fx::ping_pong(
                    fx::hsl_shift_fg(
                        [0.0, 0.0, 3.0],  // very subtle lightness shift
                        (3000, Interpolation::SineInOut),
                    )
                )
            )
        )
    }

    /// Create a tab transition effect — sweep in from left
    pub fn trigger_tab_transition() -> Effect {
        fx::parallel(&[
            fx::sweep_in(
                fx::Direction::LeftToRight,
                15,
                5,
                Color::Rgb(0, 0, 0),
                (350, Interpolation::CubicOut),
            ),
            fx::fade_from(
                Color::Rgb(0, 180, 220),
                C_BG,
                (250, Interpolation::QuadOut),
            ),
        ])
    }

    /// Create a modal open effect — coalesce from noise
    pub fn trigger_modal_open() -> Effect {
        fx::parallel(&[
            fx::coalesce((400, Interpolation::CubicOut)),
            fx::fade_from(
                C_ACCENT,
                Color::Rgb(5, 5, 12),
                (300, Interpolation::QuadOut),
            ),
        ])
    }

    /// Process effects for a frame. Call after all widgets have been rendered.
    pub fn process_frame(
        &mut self,
        f: &mut ratatui::Frame,
        header_area: Rect,
        content_area: Rect,
        statusbar_area: Rect,
        modal_area: Option<Rect>,
        current_tab_idx: usize,
        has_modal: bool,
    ) {
        let dt = self.delta();

        // ── Startup sweep ────────────────────────────────────────────────
        if let Some(ref mut fx) = self.startup_fx {
            f.render_effect(fx, f.area(), dt);
            if fx.done() {
                self.startup_fx = None;
            }
        }

        // ── Header glitch (always running) ───────────────────────────────
        f.render_effect(&mut self.header_glitch, header_area, dt);

        // ── Tab transition detection & rendering ─────────────────────────
        if current_tab_idx != self.last_tab_idx {
            self.tab_transition = Some(Self::trigger_tab_transition());
            self.last_tab_idx = current_tab_idx;
        }
        if let Some(ref mut fx) = self.tab_transition {
            f.render_effect(fx, content_area, dt);
            if fx.done() {
                self.tab_transition = None;
            }
        }

        // ── Modal open detection & rendering ─────────────────────────────
        if has_modal && !self.had_modal {
            self.modal_fx = Some(Self::trigger_modal_open());
        }
        self.had_modal = has_modal;
        if let Some(modal_rect) = modal_area {
            if let Some(ref mut fx) = self.modal_fx {
                f.render_effect(fx, modal_rect, dt);
                if fx.done() {
                    self.modal_fx = None;
                }
            }
        }

        // ── Status bar pulse (always running) ────────────────────────────
        f.render_effect(&mut self.statusbar_pulse, statusbar_area, dt);

        // ── Ambient flicker (always running, subtle) ────────────────────
        f.render_effect(&mut self.ambient_flicker, content_area, dt);
    }
}

use tachyonfx::IntoEffect;
