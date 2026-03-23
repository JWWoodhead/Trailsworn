use bevy::prelude::*;

/// Central color theme for the entire UI. Based on "The Gritty Chronicle" design system.
#[derive(Resource)]
pub struct Theme {
    // Surfaces
    pub surface: Color,
    pub surface_bright: Color,

    // Primary (gold)
    pub primary: Color,
    pub primary_container: Color,

    // Secondary (blood)
    pub secondary: Color,
    pub secondary_variant: Color,

    // Text
    pub text_parchment: Color,
    pub text_on_primary: Color,

    // Health bar
    pub hp_full: Color,
    pub hp_mid: Color,
    pub hp_low: Color,
    pub hp_bar_bg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            surface: Color::srgb(0.075, 0.075, 0.075),            // #131313
            surface_bright: Color::srgb(0.15, 0.15, 0.15),

            primary: Color::srgb(0.949, 0.792, 0.314),            // #f2ca50
            primary_container: Color::srgb(0.831, 0.686, 0.216),  // #d4af37

            secondary: Color::srgb(1.0, 0.706, 0.659),            // #ffb4a8
            secondary_variant: Color::srgb(0.573, 0.027, 0.012),  // #920703

            text_parchment: Color::srgb(0.961, 0.961, 0.863),     // #F5F5DC
            text_on_primary: Color::srgb(0.075, 0.075, 0.075),    // #131313

            hp_full: Color::srgb(0.949, 0.792, 0.314),            // #f2ca50
            hp_mid: Color::srgb(0.831, 0.686, 0.216),             // #d4af37
            hp_low: Color::srgb(0.573, 0.027, 0.012),             // #920703
            hp_bar_bg: Color::srgba(0.075, 0.075, 0.075, 0.85),
        }
    }
}

impl Theme {
    /// Lerp between two colors.
    pub fn lerp_color(a: Color, b: Color, t: f32) -> Color {
        let a = a.to_srgba();
        let b = b.to_srgba();
        Color::srgba(
            a.red + (b.red - a.red) * t,
            a.green + (b.green - a.green) * t,
            a.blue + (b.blue - a.blue) * t,
            a.alpha + (b.alpha - a.alpha) * t,
        )
    }

    /// Get HP bar color for a given health fraction (0.0 to 1.0).
    pub fn hp_color(&self, fraction: f32) -> Color {
        if fraction > 0.5 {
            let t = (fraction - 0.5) * 2.0;
            Self::lerp_color(self.hp_mid, self.hp_full, t)
        } else {
            let t = fraction * 2.0;
            Self::lerp_color(self.hp_low, self.hp_mid, t)
        }
    }
}
