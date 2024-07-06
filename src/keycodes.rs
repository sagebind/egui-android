use android_activity::input::Keycode;
use egui::Key;
use std::sync::OnceLock;

macro_rules! populate_key_map {
    ($vec:expr, {
        $(
            $from:ident => $to:ident
        ),*$(,)?
    }) => {
        $(
            $vec[u32::from(Keycode::$from) as usize] = Some(Key::$to);
        )*
    };
}

macro_rules! populate_key_map_same_name {
    ($vec:expr, {
        $(
            $ident:ident
        ),*$(,)?
    }) => {
        $(
            $vec[u32::from(Keycode::$ident) as usize] = Some(Key::$ident);
        )*
    };
}

pub(crate) fn to_physical_key(keycode: Keycode) -> Option<Key> {
    static PHYSICAL_KEY_MAP: OnceLock<Vec<Option<Key>>> = OnceLock::new();

    let map = PHYSICAL_KEY_MAP.get_or_init(|| {
        let mut map = vec![None; 256];

        populate_key_map_same_name!(map, {
            A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
            F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
        });

        populate_key_map!(map, {
            Numpad0 => Num0,
            Numpad1 => Num1,
            Numpad2 => Num2,
            Numpad3 => Num3,
            Numpad4 => Num4,
            Numpad5 => Num5,
            Numpad6 => Num6,
            Numpad7 => Num7,
            Numpad8 => Num8,
            Numpad9 => Num9,
            NumpadSubtract => Minus,
            NumpadEquals => Equals,
        });

        map
    });

    map.get(u32::from(keycode) as usize).cloned().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_key() {
        assert_eq!(to_physical_key(Keycode::R), Some(Key::R));
    }
}
