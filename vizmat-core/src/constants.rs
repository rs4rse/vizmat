use bevy::prelude::*;

// Get color for different elements
pub(crate) fn get_element_color(element: &str) -> Color {
    match element.to_uppercase().as_str() {
        "H" => Color::srgb(1.0, 1.0, 1.0),     // Hydrogen - white
        "C" => Color::srgb(0.0, 0.0, 0.0),     // Carbon - black
        "N" => Color::srgb(0.0, 0.0, 1.0),     // Nitrogen - blue
        "O" => Color::srgb(1.0, 0.0, 0.0),     // Oxygen - red
        "S" => Color::srgb(1.0, 1.0, 0.0),     // Sulfur - yellow
        "P" => Color::srgb(1.0, 0.65, 0.0),    // Phosphorus - orange
        "CL" => Color::srgb(0.0, 1.0, 0.0),    // Chlorine - green
        "BR" => Color::srgb(0.65, 0.16, 0.16), // Bromine - dark red
        "I" => Color::srgb(0.58, 0.0, 0.58),   // Iodine - purple
        "FE" => Color::srgb(1.0, 0.65, 0.0),   // Iron - orange
        "ZN" => Color::srgb(0.49, 0.50, 0.69), // Zinc - bluish
        _ => Color::srgb(0.5, 0.5, 0.5),       // Default - gray
    }
}

// Get size for different elements (van der Waals radius scaled)
pub(crate) fn get_element_size(element: &str) -> f32 {
    match element.to_uppercase().as_str() {
        "H" => 0.3,   // Hydrogen
        "C" => 0.4,   // Carbon
        "N" => 0.35,  // Nitrogen
        "O" => 0.32,  // Oxygen
        "S" => 0.45,  // Sulfur
        "P" => 0.42,  // Phosphorus
        "CL" => 0.4,  // Chlorine
        "BR" => 0.45, // Bromine
        "I" => 0.5,   // Iodine
        "FE" => 0.4,  // Iron
        "ZN" => 0.35, // Zinc
        _ => 0.35,    // Default
    }
}
