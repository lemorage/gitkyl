//! Avatar generation for author profiles
//!
//! Glass-style avatars with frosted shapes and soft pastel backgrounds.

use maud::{Markup, PreEscaped, html};

const COLORS: &[&str] = &[
    // Pinks
    "#dc8a78", "#dd7878", "#ea76cb", "#f4b8e4", "#f5c2e7", "#eba0ac", "#f2cdcd", "#fcc2d7",
    // Purples
    "#ca9ee6", "#cba6f7", "#b4befe", "#babbf1", "#c4a7e7", "#d4c1ec", "#dcc6f0", "#e2d1f5",
    // Blues
    "#8caaee", "#85c1dc", "#89dceb", "#99d1db", "#74c7ec", "#89b4fa", "#a4c8f0", "#b0d4f7",
    // Teals
    "#81c8be", "#94e2d5", "#a6e3d8", "#b5ead7", "#99e9c2", "#a3e8e0", "#afe9e4", "#b8ece7",
    // Greens
    "#a6d189", "#b4e197", "#c6d57e", "#d1e58a", "#c9e4a5", "#bde0a6", "#cde8b4", "#d5ecc0",
    // Peaches
    "#e5c890", "#f5e0dc", "#ef9f76", "#fab387", "#f9cb8c", "#f9e2af", "#fcebc4", "#fef1d6",
];

const SHAPES: &[&str] = &[
    "M52 26a10 10 0 0 0-7-7q-2-1-4-1-5 0-9 3a15 15 0 0 0-5 7q-2 5-2 11 0 7 2 11 2 5 5 7a14 14 0 0 0 9 3q4 0 7-2 3-1 5-4t1-6h-14v-14h32v10q0 10-4 17a28 28 0 0 1-12 11q-7 4-17 4-11 0-19-5t-12-13-5-20a44 44 0 0 1 3-16 34 34 0 0 1 19-19q6-3 14-3 6 0 12 2 6 2 10 5a25 25 0 0 1 10 18z",
    "M9 74V2h31q8 0 14 3 6 3 10 8a25 25 0 0 1 3 14q0 8-4 13a21 21 0 0 1-10 8q-6 3-14 3h-15v-15h8q3 0 6-1a7 7 0 0 0 4-3 8 8 0 0 0 1-5q0-3-1-5a8 8 0 0 0-4-3q-2-1-6-1h-8v57zm43-33 18 33h-21l-17-33z",
    "M21 74H0L24 2h27l24 72h-21l-16-52h-1zm-4-29h40v15h-40z",
    "M34 74H6V2h28q11 0 19 4a30 30 0 0 1 13 13q4 8 4 19 0 12-4 20a30 30 0 0 1-13 12 40 40 0 0 1-19 4m-9-17h8a22 22 0 0 0 9-2q4-2 6-6t2-12q0-7-2-12a12 12 0 0 0-6-6 24 24 0 0 0-10-2h-7z",
    "M47 2v72h-20V2z",
    "M11 74V2h52v16h-33v12h30v16h-30v12h33v16z",
    "M25 2v72h-19V2h19l52 76h1V2h19v72h-19L26 74h-1z",
    "M6 18V2h63v16h-22v56h-20V18z",
];

fn hash(s: &str) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    s.trim()
        .bytes()
        .fold(OFFSET, |h, b| (h ^ b as u64).wrapping_mul(PRIME))
}

/// Generate SVG avatar from name
pub fn generate_svg(name: &str, size: u32) -> String {
    let h = hash(name);
    let id = format!("{:x}", h & 0xFFFF);

    // Non-overlapping bit extraction (61 bits total)
    let bg = COLORS[(h % COLORS.len() as u64) as usize];
    let s1 = SHAPES[((h >> 5) % SHAPES.len() as u64) as usize];
    let s2 = SHAPES[((h >> 8) % SHAPES.len() as u64) as usize];

    let ox1 = ((h >> 11) % 160) as i32 - 80;
    let oy1 = ((h >> 19) % 160) as i32 - 80;
    let rot1 = ((h >> 27) % 360) as i32 - 180;

    let ox2 = ((h >> 36) % 160) as i32 - 80;
    let oy2 = ((h >> 44) % 160) as i32 - 80;
    let rot2 = ((h >> 52) % 320) as i32 - 160;

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 100 100"><defs><filter id="a{id}" x="-57" y="-57" width="214" height="214" filterUnits="userSpaceOnUse" color-interpolation-filters="sRGB"><feFlood flood-opacity="0" result="bg"/><feBlend in="SourceGraphic" in2="bg" result="shape"/><feGaussianBlur stdDeviation="16" result="blur"/></filter></defs><rect width="100" height="100" fill="{bg}"/><g style="mix-blend-mode:screen" opacity="0.6" filter="url(#a{id})"><path d="{s1}" fill="white" transform="translate({ox1},{oy1})rotate({rot1},37,38)scale(1.3)"/></g><g style="mix-blend-mode:screen" opacity="0.6" filter="url(#a{id})"><path d="{s2}" fill="white" transform="translate({ox2},{oy2})rotate({rot2},37,38)scale(1.3)"/></g></svg>"##
    )
}

/// Create inline SVG avatar element
pub fn render(name: &str, size: u32) -> Markup {
    html! { span class="avatar" { (PreEscaped(generate_svg(name, size))) } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        assert_eq!(generate_svg("test", 50), generate_svg("test", 50));
    }

    #[test]
    fn varies() {
        let a = generate_svg("alice", 50);
        let b = generate_svg("bob", 50);
        assert_ne!(a, b);
    }

    #[test]
    fn svg_valid() {
        for name in ["test", "user", "admin", "guest"] {
            let svg = generate_svg(name, 50);
            assert!(svg.starts_with("<svg"));
            assert!(svg.ends_with("</svg>"));
        }
    }
}
