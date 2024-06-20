#[derive(Debug)]
pub struct CubeState {
    facelets: [u8; 54],
}

impl CubeState {
    pub fn from_raw(raw: &[u8]) -> Self {
        Self {
            facelets: raw
                .iter()
                .copied()
                .flat_map(|x| [x & 0xf, (x & 0xF0) >> 4])
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        }
    }

    /// Returns the colors of the facelets on the face whose center piece is `center_color`.
    fn face_colors(&self, center_color: Color) -> [Color; 9] {
        let idx = center_color.state_index();
        (&self.facelets[idx..idx + 9])
            .iter()
            .map(|x| Color::from_u8(*x).unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

#[derive(Debug)]
enum Color {
    Orange,
    Red,
    Yellow,
    White,
    Green,
    Blue,
}

impl Color {
    /// Index into the 54-item long state array where this color's face starts
    fn state_index(&self) -> usize {
        match self {
            Self::White => 0,
            Self::Red => 9,
            Self::Green => 18,
            Self::Yellow => 27,
            Self::Orange => 36,
            Self::Blue => 45,
        }
    }

    fn from_u8(x: u8) -> Option<Self> {
        Some(match x {
            0 => Self::Orange,
            1 => Self::Red,
            2 => Self::Yellow,
            3 => Self::White,
            4 => Self::Green,
            5 => Self::Blue,
            _ => return None,
        })
    }

    fn emoji(&self) -> &'static str {
        match self {
            Color::Orange => "🟧",
            Color::Red => "🟥",
            Color::Yellow => "🟨",
            Color::White => "⬜",
            Color::Green => "🟩",
            Color::Blue => "🟦",
        }
    }
}

const TMPL: [&'static str; 7] = [
    "┌──┬──┬──┐",
    "│⬛│⬛│⬛",
    "├──┼──┼──┤",
    "│⬛│⬛│⬛",
    "├──┼──┼──┤",
    "│⬛│⬛│⬛",
    "└──┴──┴──┘",
];
const TMPLSPACE: &'static str = "          ";

fn print_template_line(lnr: usize, facelet_colors: [Color; 9]) {
    if TMPL[lnr].contains("⬛") {
        let x = TMPL[lnr]
            .split("⬛")
            .zip(facelet_colors.chunks(3).nth(lnr / 2).unwrap())
            .flat_map(|(a, color)| [a, color.emoji()])
            .collect::<Vec<_>>()
            .join("");

        print!("{x}│");
    } else {
        print!("{}", TMPL[lnr]);
    }
}

pub fn render_cube(state: &CubeState) {
    for i in 0..7 {
        print!("{TMPLSPACE}");
        print_template_line(i, state.face_colors(Color::White));
        println!();
    }
    for i in 0..7 {
        print_template_line(i, state.face_colors(Color::Orange));
        print_template_line(i, state.face_colors(Color::Green));
        print_template_line(i, state.face_colors(Color::Red));
        print_template_line(i, state.face_colors(Color::Blue));
        println!();
    }
    for i in 0..7 {
        print!("{TMPLSPACE}");
        print_template_line(i, state.face_colors(Color::Yellow));
        println!();
    }
}
