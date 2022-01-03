use bitflags::bitflags;

pub extern "C" fn observe_keyboard_event(modifiers: u8, keycode: u8) {
    log::trace!("Keyboard event!");
    let modifiers = Modifiers::from_bits_truncate(modifiers);
    crate::events::fire_key_press(KeyCode(modifiers, keycode))
}

bitflags! {
    pub struct Modifiers: u8 {
        const L_CONTROL = 0b00000001;
        const L_SHIFT   = 0b00000010;
        const L_ALT     = 0b00000100;
        const L_GUI     = 0b00001000;
        const R_CONTROL = 0b00010000;
        const R_SHIFT   = 0b00100000;
        const R_ALT     = 0b01000000;
        const R_GUI     = 0b10000000;
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct KeyCode(Modifiers, u8);

impl KeyCode {
    pub fn to_char(&self) -> Option<char> {
        if self.1 >= 104 {
            return None;
        }
        let c = if self.0.intersects(Modifiers::L_SHIFT | Modifiers::R_SHIFT) {
            KEYCODE_TO_CHAR_SHIFT[self.1 as usize]
        } else {
            KEYCODE_TO_CHAR_NOSHIFT[self.1 as usize]
        };
        if c == '\0' {
            None
        } else {
            Some(c)
        }
    }
}

#[rustfmt::skip]
const KEYCODE_TO_CHAR_NOSHIFT: [char; 104] = [
    '\0', '\0'  , '\0'  , '\0', 'a' , 'b' , 'c' , 'd' ,
    'e' , 'f'   , 'g'   , 'h' , 'i' , 'j' , 'k' , 'l' ,
    'm' , 'n'   , 'o'   , 'p' , 'q' , 'r' , 's' , 't' ,
    'u' , 'v'   , 'w'   , 'x' , 'y' , 'z' , '1' , '2' ,
    '3' , '4'   , '5'   , '6' , '7' , '8' , '9' , '0' ,
    '\n', '\x1b', '\x08', '\t', ' ' , '-' , '=' , '[' ,
    ']' , '\\'  , '#'   , ';' , '\'', '`' , ',' , '.' ,
    '/' , '\0'  , '\0'  , '\0', '\0', '\0', '\0', '\0',
    '\0', '\0'  , '\0'  , '\0', '\0', '\0', '\0', '\0',
    '\0', '\0'  , '\0'  , '\0', '\0', '\0', '\0', '\0',
    '\0', '\0'  , '\0'  , '\0', '/' , '*' , '-' , '+' ,
    '\n', '1'   , '2'   , '3' , '4' , '5' , '6' , '7' ,
    '8' , '9'   , '0'   , '.' , '\\', '\0', '\0', '=' ,
];

#[rustfmt::skip]
const   KEYCODE_TO_CHAR_SHIFT: [char; 104] = [
    '\0', '\0'  , '\0'  , '\0', 'A' , 'B' , 'C' , 'D' ,
    'E' , 'F'   , 'G'   , 'H' , 'I' , 'J' , 'K' , 'L' ,
    'M' , 'N'   , 'O'   , 'P' , 'Q' , 'R' , 'S' , 'T' ,
    'U' , 'V'   , 'W'   , 'X' , 'Y' , 'Z' , '!' , '@' ,
    '#' , '$'   , '%'   , '^' , '&' , '*' , '(' , ')' ,
    '\n', '\x1b', '\x08', '\t', ' ' , '_' , '+' , '{' ,
    '}' , '|'   , '~'   , ':' , '"' , '~' , '<' , '>' ,
    '?' , '\0'  , '\0'  , '\0', '\0', '\0', '\0', '\0',
    '\0', '\0'  , '\0'  , '\0', '\0', '\0', '\0', '\0',
    '\0', '\0'  , '\0'  , '\0', '\0', '\0', '\0', '\0',
    '\0', '\0'  , '\0'  , '\0', '/' , '*' , '-' , '+' ,
    '\n', '1'   , '2'   , '3' , '4' , '5' , '6' , '7' ,
    '8' , '9'   , '0'   , '.' , '\\', '\0', '\0', '=' ,
];
