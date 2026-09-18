/*!
 * Coloring and palette logic
 */

/// A simple RGB color
pub type Color = [u8; 3];

/// A Palette is just 4 colors (NES inspired)
pub type Palette = [Color; 4];

/// generate a random palette
pub fn random_palette() -> Palette {
    let mut palette = [[0; 3]; 4];

    for color in &mut palette {
        *color = [rand::random(), rand::random(), rand::random()];
    }

    // sort colors by brightness (our tilemap is)
    // https://stackoverflow.com/questions/20213421/sorting-color-selections-on-brightness
    palette.sort_by_key(|[r, g, b]| {
        299 * *r as u32 + 587 * *g as u32 + 114 * *b as u32
    });

    palette
}
