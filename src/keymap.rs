#![allow(dead_code)] // macros are treated as dead code sometimes
use rmk::{a, k, layer, mo, shifted, types::action::KeyAction};
pub(crate) const COLS: usize = 6;
pub(crate) const ROWS: usize = 5;

// 3-character-wide key action aliases
const ___: KeyAction = a!(Transparent);
const _0_: KeyAction = k!(Kc0);
const _1_: KeyAction = k!(Kc1);
const _2_: KeyAction = k!(Kc2);
const _3_: KeyAction = k!(Kc3);
const _4_: KeyAction = k!(Kc4);
const _5_: KeyAction = k!(Kc5);
const _6_: KeyAction = k!(Kc6);
const _7_: KeyAction = k!(Kc7);
const _8_: KeyAction = k!(Kc8);
const _9_: KeyAction = k!(Kc9);
const _A_: KeyAction = k!(A);
const _B_: KeyAction = k!(B);
const _C_: KeyAction = k!(C);
const _D_: KeyAction = k!(D);
const _E_: KeyAction = k!(E);
const _F_: KeyAction = k!(F);
const _G_: KeyAction = k!(G);
const _H_: KeyAction = k!(H);
const _I_: KeyAction = k!(I);
const _J_: KeyAction = k!(J);
const _K_: KeyAction = k!(K);
const _L_: KeyAction = k!(L);
const _M_: KeyAction = k!(M);
const _N_: KeyAction = k!(N);
const _O_: KeyAction = k!(O);
const _P_: KeyAction = k!(P);
const _Q_: KeyAction = k!(Q);
const _R_: KeyAction = k!(R);
const _S_: KeyAction = k!(S);
const _T_: KeyAction = k!(T);
const _U_: KeyAction = k!(U);
const _V_: KeyAction = k!(V);
const _W_: KeyAction = k!(W);
const _X_: KeyAction = k!(X);
const _Y_: KeyAction = k!(Y);
const _Z_: KeyAction = k!(Z);
const AMP: KeyAction = shifted!(Kc7);
const AST: KeyAction = shifted!(Kc8);
const AT_: KeyAction = shifted!(Kc2);
const BLO: KeyAction = k!(Bootloader);
const BNG: KeyAction = shifted!(Kc1);
const BSL: KeyAction = k!(Backslash);
const BSP: KeyAction = k!(Backspace);
const BTK: KeyAction = k!(Grave);
const COM: KeyAction = k!(Comma);
const CRC: KeyAction = shifted!(Kc6);
const DEL: KeyAction = k!(Delete);
const DLR: KeyAction = shifted!(Kc4);
const DOT: KeyAction = k!(Dot);
const DWN: KeyAction = k!(Down);
const END: KeyAction = k!(End);
const ENT: KeyAction = k!(Enter);
const EQL: KeyAction = k!(Equal);
const ESC: KeyAction = k!(Escape);
const F01: KeyAction = k!(F1);
const F02: KeyAction = k!(F2);
const F03: KeyAction = k!(F3);
const F04: KeyAction = k!(F4);
const F05: KeyAction = k!(F5);
const F06: KeyAction = k!(F6);
const F07: KeyAction = k!(F7);
const F08: KeyAction = k!(F8);
const F09: KeyAction = k!(F9);
const F10: KeyAction = k!(F10);
const F11: KeyAction = k!(F11);
const F12: KeyAction = k!(F12);
const F13: KeyAction = k!(F13);
const F14: KeyAction = k!(F14);
const F15: KeyAction = k!(F15);
const F16: KeyAction = k!(F16);
const F17: KeyAction = k!(F17);
const F18: KeyAction = k!(F18);
const F19: KeyAction = k!(F19);
const F20: KeyAction = k!(F20);
const F21: KeyAction = k!(F21);
const F22: KeyAction = k!(F22);
const F23: KeyAction = k!(F23);
const F24: KeyAction = k!(F24);
const GRV: KeyAction = shifted!(Grave);
const HOM: KeyAction = k!(Home);
const HSH: KeyAction = shifted!(Kc3);
const LAL: KeyAction = k!(LAlt);
const LCB: KeyAction = shifted!(LeftBracket);
const LCT: KeyAction = k!(LCtrl);
const LFT: KeyAction = k!(Left);
const LGU: KeyAction = k!(LGui);
const LOW: KeyAction = mo!(1);
const LPR: KeyAction = shifted!(Kc9);
const LSB: KeyAction = k!(LeftBracket);
const LSH: KeyAction = k!(LShift);
const MNS: KeyAction = k!(Minus);
const NXT: KeyAction = k!(MediaNextTrack);
const PCT: KeyAction = shifted!(Kc5);
const PGD: KeyAction = k!(PageDown);
const PGU: KeyAction = k!(PageUp);
const PIP: KeyAction = shifted!(Backslash);
const PLS: KeyAction = shifted!(Equal);
const PLY: KeyAction = k!(MediaPlayPause);
const PRT: KeyAction = k!(PrintScreen);
const PRV: KeyAction = k!(MediaPrevTrack);
const QUO: KeyAction = k!(Quote);
const RAI: KeyAction = mo!(2);
const RCB: KeyAction = shifted!(RightBracket);
const RGT: KeyAction = k!(Right);
const RGU: KeyAction = k!(RGui);
const RPR: KeyAction = shifted!(Kc0);
const RSB: KeyAction = k!(RightBracket);
const RSH: KeyAction = k!(RShift);
const SCN: KeyAction = k!(Semicolon);
const SLS: KeyAction = k!(Slash);
const SPC: KeyAction = k!(Space);
const TAB: KeyAction = k!(Tab);
const UP_: KeyAction = k!(Up);
const VLD: KeyAction = k!(AudioVolDown);
const VLU: KeyAction = k!(AudioVolUp);
const XXX: KeyAction = a!(No);

pub const NUM_LAYERS: usize = 3;

// Internally the peripheral board is flipped and treated like a vertical extension of the first board.
// This macro allows us to specify the keymap in an order that matches the physical layout, since the
// RMK Rust API lacks something akin to the `matrix_map` `keyboard.toml` config option.
macro_rules! lily_layer {
    (
        // ______________________________________________________________________________ row 1 _______________________________________________________________________________
        $l1_1:ident $l1_2:ident $l1_3:ident $l1_4:ident $l1_5:ident $l1_6:ident                         $r1_6:ident $r1_5:ident $r1_4:ident $r1_3:ident $r1_2:ident $r1_1:ident
        // ______________________________________________________________________________ row 2 _______________________________________________________________________________
        $l2_1:ident $l2_2:ident $l2_3:ident $l2_4:ident $l2_5:ident $l2_6:ident                         $r2_6:ident $r2_5:ident $r2_4:ident $r2_3:ident $r2_2:ident $r2_1:ident
        // ______________________________________________________________________________ row 3 _______________________________________________________________________________
        $l3_1:ident $l3_2:ident $l3_3:ident $l3_4:ident $l3_5:ident $l3_6:ident                         $r3_6:ident $r3_5:ident $r3_4:ident $r3_3:ident $r3_2:ident $r3_1:ident
        // ______________________________________________________________________________ row 4 _______________________________________________________________________________
        $l4_1:ident $l4_2:ident $l4_3:ident $l4_4:ident $l4_5:ident $l4_6:ident $t_l7:ident $t_r2:ident $r4_6:ident $r4_5:ident $r4_4:ident $r4_3:ident $r4_2:ident $r4_1:ident
        // ______________________________________________________________________________ thumbs ______________________________________________________________________________
                                            $t_l3:ident $t_l4:ident $t_l5:ident $t_l6:ident $t_r6:ident $t_r5:ident $t_r4:ident $t_r3:ident
    ) => {
        layer!([
            [$l1_1, $l1_2, $l1_3, $l1_4, $l1_5, $l1_6],
            [$l2_1, $l2_2, $l2_3, $l2_4, $l2_5, $l2_6],
            [$l3_1, $l3_2, $l3_3, $l3_4, $l3_5, $l3_6],
            [$l4_1, $l4_2, $l4_3, $l4_4, $l4_5, $l4_6],
            [XXX, $t_l3, $t_l4, $t_l5, $t_l6, $t_l7],
            [$r1_1, $r1_2, $r1_3, $r1_4, $r1_5, $r1_6],
            [$r2_1, $r2_2, $r2_3, $r2_4, $r2_5, $r2_6],
            [$r3_1, $r3_2, $r3_3, $r3_4, $r3_5, $r3_6],
            [$r4_1, $r4_2, $r4_3, $r4_4, $r4_5, $r4_6],
            [XXX, $t_r3, $t_r4, $t_r5, $t_r6, $t_r2]
        ])
    };
}

pub const fn get_default_keymap() -> [[[KeyAction; COLS]; ROWS * 2]; NUM_LAYERS] {
    [
        lily_layer!(
            ESC _1_ _2_ _3_ _4_ _5_         _6_ _7_ _8_ _9_ _0_ EQL
            TAB _Q_ _W_ _E_ _R_ _T_         _Y_ _U_ _I_ _O_ _P_ MNS
            LSH _A_ _S_ _D_ _F_ _G_         _H_ _J_ _K_ _L_ SCN QUO
            LCT _Z_ _X_ _C_ _V_ _B_ HOM PGU _N_ _M_ COM DOT SLS RSH
                        LAL LGU LOW SPC ENT RAI BSP RGU
        ),
        lily_layer!(
            F01 F02 F03 F04 F05 F06         F07 F08 F09 F10 F11 F12
            TAB XXX XXX XXX XXX XXX         XXX XXX XXX XXX XXX MNS
            LSH BNG AT_ HSH DLR PCT         CRC AMP AST LPR RPR BSL
            LCT XXX XXX XXX XXX BTK END PGD GRV LSB RSB LCB RCB PIP
                        LAL LGU LOW BLO ENT RAI DEL RGU
        ),
        lily_layer!(
            F13 F14 F15 F16 F17 F18         F19 F20 F21 F22 F23 F24
            ___ PLS MNS AST SLS EQL         PLY PRV VLD VLU NXT PRT
            ___ _1_ _2_ _3_ _4_ _5_         HOM LFT DWN UP_ RGT END
            ___ _6_ _7_ _8_ _9_ _0_ DOT LPR _N_ _M_ COM DOT SLS RSH
                        ___ ___ ___ SPC ___ ___ BSP ___
        ),
    ]
}
