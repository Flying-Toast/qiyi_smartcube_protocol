/////////////////////////////////
// TODO clean this crap up :-) //
/////////////////////////////////

#[derive(Debug)]
pub struct CubeState<'a> {
    raw: &'a [u8],
}

impl<'a> CubeState<'a> {
    pub fn from_raw(raw: &'a [u8]) -> Self {
        Self { raw }
    }

    fn get_white_colors(&self) -> [u8; 9] {
        let mut b = splitbytes(&self.raw[0..0 + 5]);
        b.remove(8);
        b.try_into().unwrap()
    }

    fn get_red_colors(&self) -> [u8; 9] {
        let mut b = splitbytes(&self.raw[4..4 + 5]);
        b.remove(1);
        b.try_into().unwrap()
    }

    fn get_green_colors(&self) -> [u8; 9] {
        let mut b = splitbytes(&self.raw[9..9 + 5]);
        b.remove(8);
        b.try_into().unwrap()
    }

    fn get_yellow_colors(&self) -> [u8; 9] {
        let mut b = splitbytes(&self.raw[13..13 + 5]);
        b.remove(1);
        b.try_into().unwrap()
    }

    fn get_orange_colors(&self) -> [u8; 9] {
        let mut b = splitbytes(&self.raw[18..18 + 5]);
        b.remove(8);
        b.try_into().unwrap()
    }

    fn get_blue_colors(&self) -> [u8; 9] {
        let mut b = splitbytes(&self.raw[22..22 + 5]);
        b.remove(1);
        b.try_into().unwrap()
    }
}

fn splitbytes(bytes: &[u8]) -> Vec<u8> {
    bytes
        .iter()
        .flat_map(|x| [(x & 0xF0) >> 4, x & 0xF])
        .collect()
}

const ORANGE: u8 = 0;
const RED: u8 = 1;
const YELLOW: u8 = 2;
const WHITE: u8 = 3;
const GREEN: u8 = 4;
const BLUE: u8 = 5;

fn to_emoji(x: u8) -> &'static str {
    if x == ORANGE {
        "🟧"
    } else if x == RED {
        "🟥"
    } else if x == YELLOW {
        "🟨"
    } else if x == WHITE {
        "⬜"
    } else if x == GREEN {
        "🟩"
    } else if x == BLUE {
        "🟦"
    } else {
        panic!()
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

fn print_template_line(lnr: usize, layout: [[usize; 3]; 3], colormap: [u8; 9]) {
    if lnr > 0 && (lnr + 1) % 2 == 0 {
        let rnr = (lnr + 1) / 2 - 1;
        let x = TMPL[lnr]
            .split("⬛")
            .zip(layout[rnr])
            .flat_map(|(a, b)| [a, to_emoji(colormap[b])])
            .collect::<Vec<_>>()
            .join("");

        print!("{x}│");
    } else {
        print!("{}", TMPL[lnr]);
    }
}

pub fn render_cube(state: &CubeState) {
    #[rustfmt::skip]
    let blue = [
        [7,8,5],
        [6,3,4],
        [1,2,0],
    ];
    #[rustfmt::skip]
    let orange = [
        [7,2,1],
        [6,5,0],
        [8,4,3],
    ];
    #[rustfmt::skip]
    let white = [
        [1,0,3],
        [2,5,4],
        [7,6,8],
    ];
    #[rustfmt::skip]
    let red = [
        [1,6,7],
        [2,3,8],
        [0,4,5],
    ];
    let green = white.clone();
    #[rustfmt::skip]
    let yellow = [
        [0,2,1],
        [4,3,6],
        [5,8,7],
    ];

    for i in 0..7 {
        print!("{TMPLSPACE}");
        print_template_line(i, blue, state.get_blue_colors());
        println!();
    }
    for i in 0..7 {
        print_template_line(i, orange, state.get_orange_colors());
        print_template_line(i, white, state.get_white_colors());
        print_template_line(i, red, state.get_red_colors());
        println!();
    }
    for i in 0..7 {
        print!("{TMPLSPACE}");
        print_template_line(i, green, state.get_green_colors());
        println!();
    }
    for i in 0..7 {
        print!("{TMPLSPACE}");
        print_template_line(i, yellow, state.get_yellow_colors());
        println!();
    }
}
