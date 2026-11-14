// Generated from libxkbcommon xkbcommon-keysyms.h; regenerate with scripts/windows/generate-keysyms.py.
/***********************************************************
Copyright 1987, 1994, 1998  The Open Group

Permission to use, copy, modify, distribute, and sell this software and its
documentation for any purpose is hereby granted without fee, provided that
the above copyright notice appear in all copies and that both that
copyright notice and this permission notice appear in supporting
documentation.

The above copyright notice and this permission notice shall be included
in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
IN NO EVENT SHALL THE OPEN GROUP BE LIABLE FOR ANY CLAIM, DAMAGES OR
OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
OTHER DEALINGS IN THE SOFTWARE.

Except as contained in this notice, the name of The Open Group shall
not be used in advertising or otherwise to promote the sale, use or
other dealings in this Software without prior written authorization
from The Open Group.


Copyright 1987 by Digital Equipment Corporation, Maynard, Massachusetts

                        All Rights Reserved

Permission to use, copy, modify, and distribute this software and its
documentation for any purpose and without fee is hereby granted,
provided that the above copyright notice appear in all copies and that
both that copyright notice and this permission notice appear in
supporting documentation, and that the name of Digital not be
used in advertising or publicity pertaining to distribution of the
software without specific, written prior permission.

DIGITAL DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS SOFTWARE, INCLUDING
ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS, IN NO EVENT SHALL
DIGITAL BE LIABLE FOR ANY SPECIAL, INDIRECT OR CONSEQUENTIAL DAMAGES OR
ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS
SOFTWARE.

******************************************************************/

/*
 * The "X11 Window System Protocol" standard defines in Appendix A the
 * keysym codes. These 29-bit integer values identify characters or
 * functions associated with each key (e.g., via the visible
 * engraving) of a keyboard layout. This file assigns mnemonic macro
 * names for these keysyms.
 *
 * This file is also compiled (by src/util/makekeys.c in libX11) into
 * hash tables that can be accessed with X11 library functions such as
 * XStringToKeysym() and XKeysymToString().
 *
 * Where a keysym corresponds one-to-one to an ISO 10646 / Unicode
 * character, this is noted in a comment that provides both the U+xxxx
 * Unicode position, as well as the official Unicode name of the
 * character.
 *
 * Some keysyms map to a character already mapped by another keysym,
 * with compatible but more precise semantics, such as the keypad-
 * related keysyms. In this case, none of the keysym are deprecated.
 * The most generic keysym is annotated as previously and more specific
 * keysyms have the same annotation between angle brackets:
 *
 *     #define XKB_KEY_space                 0x0020  // U+0020 SPACE
 *     #define XKB_KEY_KP_Space              0xff80  //<U+0020 SPACE>
 *
 * Where the correspondence is either not one-to-one or semantically
 * unclear, the Unicode position and name are enclosed in
 * parentheses. Such legacy keysyms should be considered deprecated
 * and are not recommended for use in future keyboard mappings.
 *
 * For any future extension of the keysyms with characters already
 * found in ISO 10646 / Unicode, the following algorithm shall be
 * used. The new keysym code position will simply be the character's
 * Unicode number plus 0x01000000. The keysym values in the range
 * 0x01000100 to 0x0110ffff are reserved to represent Unicode
 * characters in the range U+0100 to U+10FFFF.
 *
 * While most newer Unicode-based X11 clients do already accept
 * Unicode-mapped keysyms in the range 0x01000100 to 0x0110ffff, it
 * will remain necessary for clients -- in the interest of
 * compatibility with existing servers -- to also understand the
 * existing legacy keysym values in the range 0x0100 to 0x20ff.
 *
 * Where several mnemonic names are defined for the same keysym in this
 * file, the first one listed is considered the "canonical" name. This
 * is the name that should be used when retrieving a keysym name from
 * its code. The next names are considered "aliases" to the canonical
 * name.
 *
 * Aliases are made explicit by writing in their comment "alias for",
 * followed by the corresponding canonical name. Example:
 *
 *     #define XKB_KEY_dead_tilde            0xfe53
 *     #define XKB_KEY_dead_perispomeni      0xfe53 // alias for dead_tilde
 *
 * The rules to consider a keysym mnemonic name deprecated are:
 *
 *   1. A legacy keysym with its Unicode mapping in parentheses is
 *      deprecated (see above).
 *
 *   2. A keysym name is *explicitly* deprecated by starting its comment
 *      with "deprecated". Examples:
 *
 *        #define XKB_KEY_L1           0xffc8  // deprecated alias for F11
 *        #define XKB_KEY_quoteleft    0x0060  // deprecated
 *
 *   3. A keysym name is *explicitly* *not* deprecated by starting its
 *      comment with "non-deprecated alias". Examples:
 *
 *       #define XKB_KEY_dead_tilde       0xfe53
 *       #define XKB_KEY_dead_perispomeni 0xfe53 // non-deprecated alias for dead_tilde
 *
 *   4. If none of the previous rules apply, an alias is *implicitly*
 *      deprecated if there is at least one previous name for the
 *      corresponding keysym that is *not* explicitly deprecated.
 *
 *      Examples:
 *
 *        // SingleCandidate is the canonical name
 *        #define XKB_KEY_SingleCandidate        0xff3c
 *        // Hangul_SingleCandidate is deprecated because it is an alias
 *        // and it does not start with "non-deprecated alias"
 *        #define XKB_KEY_Hangul_SingleCandidate 0xff3c // Single candidate
 *
 *        // guillemotleft is the canonical name, but it is deprecated
 *        #define XKB_KEY_guillemotleft  0x00ab // deprecated alias for guillemetleft (misspelling)
 *        // guillemetleft is not deprecated, because the keysym has no endorsed name before it.
 *        #define XKB_KEY_guillemetleft  0x00ab // U+00AB LEFT-POINTING DOUBLE ANGLE QUOTATION MARK
 *        // The following hypothetical name is deprecated because guillemetleft come before.
 *        #define XKB_KEY_guillemetleft2 0x00ab
 *
 * Mnemonic names for keysyms are defined in this file with lines
 * that match one of these Perl regular expressions:
 *
 *    /^\#define XKB_KEY_([a-zA-Z_0-9]+)\s+0x([0-9a-f]+)\s*\/\* U\+([0-9A-F]{4,6}) (.*) \*\/\s*$/
 *    /^\#define XKB_KEY_([a-zA-Z_0-9]+)\s+0x([0-9a-f]+)\s*\/\*<U\+([0-9A-F]{4,6}) (.*)>\*\/\s*$/
 *    /^\#define XKB_KEY_([a-zA-Z_0-9]+)\s+0x([0-9a-f]+)\s*\/\*\(U\+([0-9A-F]{4,6}) (.*)\)\*\/\s*$/
 *    /^\#define XKB_KEY_([a-zA-Z_0-9]+)\s+0x([0-9a-f]+)\s*(\/\*\s*(.*)\s*\*\/)?\s*$/
 *
 * Before adding new keysyms, please do consider the following: In
 * addition to the keysym names defined in this file, the
 * XStringToKeysym() and XKeysymToString() functions will also handle
 * any keysym string of the form "U0020" to "U007E" and "U00A0" to
 * "U10FFFF" for all possible Unicode characters. In other words,
 * every possible Unicode character has already a keysym string
 * defined algorithmically, even if it is not listed here. Therefore,
 * defining an additional keysym macro is only necessary where a
 * non-hexadecimal mnemonic name is needed, or where the new keysym
 * does not represent any existing Unicode character.
 *
 * When adding new keysyms to this file, do not forget to also update the
 * following as needed:
 *
 *   - the mappings in src/KeyBind.c in the libX11 repo
 *     https://gitlab.freedesktop.org/xorg/lib/libx11
 *
 *   - the protocol specification in specs/keysyms.xml in this repo
 *     https://gitlab.freedesktop.org/xorg/proto/xorgproto
 *
 * Before removing or changing the order of the keysyms, please consider
 * the following: it is very difficult to know what keysyms are used and
 * how.
 *
 *   - A sandboxed application may have incompatibilities with the host
 *     system. For example, if new keysym name is introduced and is made
 *     the canonical name, then an application with an older keysym parser
 *     will not be able to parse the new name.
 *   - Customization of keyboard layout and Compose files are two popular
 *     use cases. Checking the standard keyboard layout database xkeyboard-config
 *     https://gitlab.freedesktop.org/xkeyboard-config/xkeyboard-config
 *     and the standard Compose files in libx11
 *     https://gitlab.freedesktop.org/xorg/lib/libx11 is a mandatory
 *     step, but may *not* be enough for a proper impact assessment for
 *     e.g. keysyms removals.
 *
 * Therefore, it is advised to proceed to no removal and to make a new
 * name canonical only 10 years after its introduction. This means that
 * some keysyms may have their first listed name deprecated during the
 * period of transition. Once this period is over, the deprecated name
 * should be moved after the new canonical name.
 */


#[allow(dead_code, non_upper_case_globals)]
pub mod keysyms {
    pub const KEY_NoSymbol: u32 = 0x000000;
    pub const KEY_VoidSymbol: u32 = 0xffffff;
    pub const KEY_BackSpace: u32 = 0xff08;
    pub const KEY_Tab: u32 = 0xff09;
    pub const KEY_Linefeed: u32 = 0xff0a;
    pub const KEY_Clear: u32 = 0xff0b;
    pub const KEY_Return: u32 = 0xff0d;
    pub const KEY_Pause: u32 = 0xff13;
    pub const KEY_Scroll_Lock: u32 = 0xff14;
    pub const KEY_Sys_Req: u32 = 0xff15;
    pub const KEY_Escape: u32 = 0xff1b;
    pub const KEY_Delete: u32 = 0xffff;
    pub const KEY_Multi_key: u32 = 0xff20;
    pub const KEY_Codeinput: u32 = 0xff37;
    pub const KEY_SingleCandidate: u32 = 0xff3c;
    pub const KEY_MultipleCandidate: u32 = 0xff3d;
    pub const KEY_PreviousCandidate: u32 = 0xff3e;
    pub const KEY_Kanji: u32 = 0xff21;
    pub const KEY_Muhenkan: u32 = 0xff22;
    pub const KEY_Henkan_Mode: u32 = 0xff23;
    pub const KEY_Henkan: u32 = 0xff23;
    pub const KEY_Romaji: u32 = 0xff24;
    pub const KEY_Hiragana: u32 = 0xff25;
    pub const KEY_Katakana: u32 = 0xff26;
    pub const KEY_Hiragana_Katakana: u32 = 0xff27;
    pub const KEY_Zenkaku: u32 = 0xff28;
    pub const KEY_Hankaku: u32 = 0xff29;
    pub const KEY_Zenkaku_Hankaku: u32 = 0xff2a;
    pub const KEY_Touroku: u32 = 0xff2b;
    pub const KEY_Massyo: u32 = 0xff2c;
    pub const KEY_Kana_Lock: u32 = 0xff2d;
    pub const KEY_Kana_Shift: u32 = 0xff2e;
    pub const KEY_Eisu_Shift: u32 = 0xff2f;
    pub const KEY_Eisu_toggle: u32 = 0xff30;
    pub const KEY_Kanji_Bangou: u32 = 0xff37;
    pub const KEY_Zen_Koho: u32 = 0xff3d;
    pub const KEY_Mae_Koho: u32 = 0xff3e;
    pub const KEY_Home: u32 = 0xff50;
    pub const KEY_Left: u32 = 0xff51;
    pub const KEY_Up: u32 = 0xff52;
    pub const KEY_Right: u32 = 0xff53;
    pub const KEY_Down: u32 = 0xff54;
    pub const KEY_Prior: u32 = 0xff55;
    pub const KEY_Page_Up: u32 = 0xff55;
    pub const KEY_Next: u32 = 0xff56;
    pub const KEY_Page_Down: u32 = 0xff56;
    pub const KEY_End: u32 = 0xff57;
    pub const KEY_Begin: u32 = 0xff58;
    pub const KEY_Select: u32 = 0xff60;
    pub const KEY_Print: u32 = 0xff61;
    pub const KEY_Execute: u32 = 0xff62;
    pub const KEY_Insert: u32 = 0xff63;
    pub const KEY_Undo: u32 = 0xff65;
    pub const KEY_Redo: u32 = 0xff66;
    pub const KEY_Menu: u32 = 0xff67;
    pub const KEY_Find: u32 = 0xff68;
    pub const KEY_Cancel: u32 = 0xff69;
    pub const KEY_Help: u32 = 0xff6a;
    pub const KEY_Break: u32 = 0xff6b;
    pub const KEY_ISO_Group_Shift: u32 = 0xff7e;
    pub const KEY_Mode_switch: u32 = 0xff7e;
    pub const KEY_script_switch: u32 = 0xff7e;
    pub const KEY_Num_Lock: u32 = 0xff7f;
    pub const KEY_KP_Space: u32 = 0xff80;
    pub const KEY_KP_Tab: u32 = 0xff89;
    pub const KEY_KP_Enter: u32 = 0xff8d;
    pub const KEY_KP_F1: u32 = 0xff91;
    pub const KEY_KP_F2: u32 = 0xff92;
    pub const KEY_KP_F3: u32 = 0xff93;
    pub const KEY_KP_F4: u32 = 0xff94;
    pub const KEY_KP_Home: u32 = 0xff95;
    pub const KEY_KP_Left: u32 = 0xff96;
    pub const KEY_KP_Up: u32 = 0xff97;
    pub const KEY_KP_Right: u32 = 0xff98;
    pub const KEY_KP_Down: u32 = 0xff99;
    pub const KEY_KP_Prior: u32 = 0xff9a;
    pub const KEY_KP_Page_Up: u32 = 0xff9a;
    pub const KEY_KP_Next: u32 = 0xff9b;
    pub const KEY_KP_Page_Down: u32 = 0xff9b;
    pub const KEY_KP_End: u32 = 0xff9c;
    pub const KEY_KP_Begin: u32 = 0xff9d;
    pub const KEY_KP_Insert: u32 = 0xff9e;
    pub const KEY_KP_Delete: u32 = 0xff9f;
    pub const KEY_KP_Equal: u32 = 0xffbd;
    pub const KEY_KP_Multiply: u32 = 0xffaa;
    pub const KEY_KP_Add: u32 = 0xffab;
    pub const KEY_KP_Separator: u32 = 0xffac;
    pub const KEY_KP_Subtract: u32 = 0xffad;
    pub const KEY_KP_Decimal: u32 = 0xffae;
    pub const KEY_KP_Divide: u32 = 0xffaf;
    pub const KEY_KP_0: u32 = 0xffb0;
    pub const KEY_KP_1: u32 = 0xffb1;
    pub const KEY_KP_2: u32 = 0xffb2;
    pub const KEY_KP_3: u32 = 0xffb3;
    pub const KEY_KP_4: u32 = 0xffb4;
    pub const KEY_KP_5: u32 = 0xffb5;
    pub const KEY_KP_6: u32 = 0xffb6;
    pub const KEY_KP_7: u32 = 0xffb7;
    pub const KEY_KP_8: u32 = 0xffb8;
    pub const KEY_KP_9: u32 = 0xffb9;
    pub const KEY_F1: u32 = 0xffbe;
    pub const KEY_F2: u32 = 0xffbf;
    pub const KEY_F3: u32 = 0xffc0;
    pub const KEY_F4: u32 = 0xffc1;
    pub const KEY_F5: u32 = 0xffc2;
    pub const KEY_F6: u32 = 0xffc3;
    pub const KEY_F7: u32 = 0xffc4;
    pub const KEY_F8: u32 = 0xffc5;
    pub const KEY_F9: u32 = 0xffc6;
    pub const KEY_F10: u32 = 0xffc7;
    pub const KEY_F11: u32 = 0xffc8;
    pub const KEY_L1: u32 = 0xffc8;
    pub const KEY_F12: u32 = 0xffc9;
    pub const KEY_L2: u32 = 0xffc9;
    pub const KEY_F13: u32 = 0xffca;
    pub const KEY_L3: u32 = 0xffca;
    pub const KEY_F14: u32 = 0xffcb;
    pub const KEY_L4: u32 = 0xffcb;
    pub const KEY_F15: u32 = 0xffcc;
    pub const KEY_L5: u32 = 0xffcc;
    pub const KEY_F16: u32 = 0xffcd;
    pub const KEY_L6: u32 = 0xffcd;
    pub const KEY_F17: u32 = 0xffce;
    pub const KEY_L7: u32 = 0xffce;
    pub const KEY_F18: u32 = 0xffcf;
    pub const KEY_L8: u32 = 0xffcf;
    pub const KEY_F19: u32 = 0xffd0;
    pub const KEY_L9: u32 = 0xffd0;
    pub const KEY_F20: u32 = 0xffd1;
    pub const KEY_L10: u32 = 0xffd1;
    pub const KEY_F21: u32 = 0xffd2;
    pub const KEY_R1: u32 = 0xffd2;
    pub const KEY_F22: u32 = 0xffd3;
    pub const KEY_R2: u32 = 0xffd3;
    pub const KEY_F23: u32 = 0xffd4;
    pub const KEY_R3: u32 = 0xffd4;
    pub const KEY_F24: u32 = 0xffd5;
    pub const KEY_R4: u32 = 0xffd5;
    pub const KEY_F25: u32 = 0xffd6;
    pub const KEY_R5: u32 = 0xffd6;
    pub const KEY_F26: u32 = 0xffd7;
    pub const KEY_R6: u32 = 0xffd7;
    pub const KEY_F27: u32 = 0xffd8;
    pub const KEY_R7: u32 = 0xffd8;
    pub const KEY_F28: u32 = 0xffd9;
    pub const KEY_R8: u32 = 0xffd9;
    pub const KEY_F29: u32 = 0xffda;
    pub const KEY_R9: u32 = 0xffda;
    pub const KEY_F30: u32 = 0xffdb;
    pub const KEY_R10: u32 = 0xffdb;
    pub const KEY_F31: u32 = 0xffdc;
    pub const KEY_R11: u32 = 0xffdc;
    pub const KEY_F32: u32 = 0xffdd;
    pub const KEY_R12: u32 = 0xffdd;
    pub const KEY_F33: u32 = 0xffde;
    pub const KEY_R13: u32 = 0xffde;
    pub const KEY_F34: u32 = 0xffdf;
    pub const KEY_R14: u32 = 0xffdf;
    pub const KEY_F35: u32 = 0xffe0;
    pub const KEY_R15: u32 = 0xffe0;
    pub const KEY_Shift_L: u32 = 0xffe1;
    pub const KEY_Shift_R: u32 = 0xffe2;
    pub const KEY_Control_L: u32 = 0xffe3;
    pub const KEY_Control_R: u32 = 0xffe4;
    pub const KEY_Caps_Lock: u32 = 0xffe5;
    pub const KEY_Shift_Lock: u32 = 0xffe6;
    pub const KEY_Meta_L: u32 = 0xffe7;
    pub const KEY_Meta_R: u32 = 0xffe8;
    pub const KEY_Alt_L: u32 = 0xffe9;
    pub const KEY_Alt_R: u32 = 0xffea;
    pub const KEY_Super_L: u32 = 0xffeb;
    pub const KEY_Super_R: u32 = 0xffec;
    pub const KEY_Hyper_L: u32 = 0xffed;
    pub const KEY_Hyper_R: u32 = 0xffee;
    pub const KEY_ISO_Lock: u32 = 0xfe01;
    pub const KEY_ISO_Level2_Latch: u32 = 0xfe02;
    pub const KEY_ISO_Level3_Shift: u32 = 0xfe03;
    pub const KEY_ISO_Level3_Latch: u32 = 0xfe04;
    pub const KEY_ISO_Level3_Lock: u32 = 0xfe05;
    pub const KEY_ISO_Level5_Shift: u32 = 0xfe11;
    pub const KEY_ISO_Level5_Latch: u32 = 0xfe12;
    pub const KEY_ISO_Level5_Lock: u32 = 0xfe13;
    pub const KEY_ISO_Group_Latch: u32 = 0xfe06;
    pub const KEY_ISO_Group_Lock: u32 = 0xfe07;
    pub const KEY_ISO_Next_Group: u32 = 0xfe08;
    pub const KEY_ISO_Next_Group_Lock: u32 = 0xfe09;
    pub const KEY_ISO_Prev_Group: u32 = 0xfe0a;
    pub const KEY_ISO_Prev_Group_Lock: u32 = 0xfe0b;
    pub const KEY_ISO_First_Group: u32 = 0xfe0c;
    pub const KEY_ISO_First_Group_Lock: u32 = 0xfe0d;
    pub const KEY_ISO_Last_Group: u32 = 0xfe0e;
    pub const KEY_ISO_Last_Group_Lock: u32 = 0xfe0f;
    pub const KEY_ISO_Left_Tab: u32 = 0xfe20;
    pub const KEY_ISO_Move_Line_Up: u32 = 0xfe21;
    pub const KEY_ISO_Move_Line_Down: u32 = 0xfe22;
    pub const KEY_ISO_Partial_Line_Up: u32 = 0xfe23;
    pub const KEY_ISO_Partial_Line_Down: u32 = 0xfe24;
    pub const KEY_ISO_Partial_Space_Left: u32 = 0xfe25;
    pub const KEY_ISO_Partial_Space_Right: u32 = 0xfe26;
    pub const KEY_ISO_Set_Margin_Left: u32 = 0xfe27;
    pub const KEY_ISO_Set_Margin_Right: u32 = 0xfe28;
    pub const KEY_ISO_Release_Margin_Left: u32 = 0xfe29;
    pub const KEY_ISO_Release_Margin_Right: u32 = 0xfe2a;
    pub const KEY_ISO_Release_Both_Margins: u32 = 0xfe2b;
    pub const KEY_ISO_Fast_Cursor_Left: u32 = 0xfe2c;
    pub const KEY_ISO_Fast_Cursor_Right: u32 = 0xfe2d;
    pub const KEY_ISO_Fast_Cursor_Up: u32 = 0xfe2e;
    pub const KEY_ISO_Fast_Cursor_Down: u32 = 0xfe2f;
    pub const KEY_ISO_Continuous_Underline: u32 = 0xfe30;
    pub const KEY_ISO_Discontinuous_Underline: u32 = 0xfe31;
    pub const KEY_ISO_Emphasize: u32 = 0xfe32;
    pub const KEY_ISO_Center_Object: u32 = 0xfe33;
    pub const KEY_ISO_Enter: u32 = 0xfe34;
    pub const KEY_dead_grave: u32 = 0xfe50;
    pub const KEY_dead_acute: u32 = 0xfe51;
    pub const KEY_dead_circumflex: u32 = 0xfe52;
    pub const KEY_dead_tilde: u32 = 0xfe53;
    pub const KEY_dead_perispomeni: u32 = 0xfe53;
    pub const KEY_dead_macron: u32 = 0xfe54;
    pub const KEY_dead_breve: u32 = 0xfe55;
    pub const KEY_dead_abovedot: u32 = 0xfe56;
    pub const KEY_dead_diaeresis: u32 = 0xfe57;
    pub const KEY_dead_abovering: u32 = 0xfe58;
    pub const KEY_dead_doubleacute: u32 = 0xfe59;
    pub const KEY_dead_caron: u32 = 0xfe5a;
    pub const KEY_dead_cedilla: u32 = 0xfe5b;
    pub const KEY_dead_ogonek: u32 = 0xfe5c;
    pub const KEY_dead_iota: u32 = 0xfe5d;
    pub const KEY_dead_voiced_sound: u32 = 0xfe5e;
    pub const KEY_dead_semivoiced_sound: u32 = 0xfe5f;
    pub const KEY_dead_belowdot: u32 = 0xfe60;
    pub const KEY_dead_hook: u32 = 0xfe61;
    pub const KEY_dead_horn: u32 = 0xfe62;
    pub const KEY_dead_stroke: u32 = 0xfe63;
    pub const KEY_dead_abovecomma: u32 = 0xfe64;
    pub const KEY_dead_psili: u32 = 0xfe64;
    pub const KEY_dead_abovereversedcomma: u32 = 0xfe65;
    pub const KEY_dead_dasia: u32 = 0xfe65;
    pub const KEY_dead_doublegrave: u32 = 0xfe66;
    pub const KEY_dead_belowring: u32 = 0xfe67;
    pub const KEY_dead_belowmacron: u32 = 0xfe68;
    pub const KEY_dead_belowcircumflex: u32 = 0xfe69;
    pub const KEY_dead_belowtilde: u32 = 0xfe6a;
    pub const KEY_dead_belowbreve: u32 = 0xfe6b;
    pub const KEY_dead_belowdiaeresis: u32 = 0xfe6c;
    pub const KEY_dead_invertedbreve: u32 = 0xfe6d;
    pub const KEY_dead_belowcomma: u32 = 0xfe6e;
    pub const KEY_dead_currency: u32 = 0xfe6f;
    pub const KEY_dead_lowline: u32 = 0xfe90;
    pub const KEY_dead_aboveverticalline: u32 = 0xfe91;
    pub const KEY_dead_belowverticalline: u32 = 0xfe92;
    pub const KEY_dead_longsolidusoverlay: u32 = 0xfe93;
    pub const KEY_dead_a: u32 = 0xfe80;
    pub const KEY_dead_A: u32 = 0xfe81;
    pub const KEY_dead_e: u32 = 0xfe82;
    pub const KEY_dead_E: u32 = 0xfe83;
    pub const KEY_dead_i: u32 = 0xfe84;
    pub const KEY_dead_I: u32 = 0xfe85;
    pub const KEY_dead_o: u32 = 0xfe86;
    pub const KEY_dead_O: u32 = 0xfe87;
    pub const KEY_dead_u: u32 = 0xfe88;
    pub const KEY_dead_U: u32 = 0xfe89;
    pub const KEY_dead_small_schwa: u32 = 0xfe8a;
    pub const KEY_dead_schwa: u32 = 0xfe8a;
    pub const KEY_dead_capital_schwa: u32 = 0xfe8b;
    pub const KEY_dead_SCHWA: u32 = 0xfe8b;
    pub const KEY_dead_greek: u32 = 0xfe8c;
    pub const KEY_dead_hamza: u32 = 0xfe8d;
    pub const KEY_dead_apostrophe: u32 = 0xfe8e;
    pub const KEY_First_Virtual_Screen: u32 = 0xfed0;
    pub const KEY_Prev_Virtual_Screen: u32 = 0xfed1;
    pub const KEY_Next_Virtual_Screen: u32 = 0xfed2;
    pub const KEY_Last_Virtual_Screen: u32 = 0xfed4;
    pub const KEY_Terminate_Server: u32 = 0xfed5;
    pub const KEY_AccessX_Enable: u32 = 0xfe70;
    pub const KEY_AccessX_Feedback_Enable: u32 = 0xfe71;
    pub const KEY_RepeatKeys_Enable: u32 = 0xfe72;
    pub const KEY_SlowKeys_Enable: u32 = 0xfe73;
    pub const KEY_BounceKeys_Enable: u32 = 0xfe74;
    pub const KEY_StickyKeys_Enable: u32 = 0xfe75;
    pub const KEY_MouseKeys_Enable: u32 = 0xfe76;
    pub const KEY_MouseKeys_Accel_Enable: u32 = 0xfe77;
    pub const KEY_Overlay1_Enable: u32 = 0xfe78;
    pub const KEY_Overlay2_Enable: u32 = 0xfe79;
    pub const KEY_AudibleBell_Enable: u32 = 0xfe7a;
    pub const KEY_Pointer_Left: u32 = 0xfee0;
    pub const KEY_Pointer_Right: u32 = 0xfee1;
    pub const KEY_Pointer_Up: u32 = 0xfee2;
    pub const KEY_Pointer_Down: u32 = 0xfee3;
    pub const KEY_Pointer_UpLeft: u32 = 0xfee4;
    pub const KEY_Pointer_UpRight: u32 = 0xfee5;
    pub const KEY_Pointer_DownLeft: u32 = 0xfee6;
    pub const KEY_Pointer_DownRight: u32 = 0xfee7;
    pub const KEY_Pointer_Button_Dflt: u32 = 0xfee8;
    pub const KEY_Pointer_Button1: u32 = 0xfee9;
    pub const KEY_Pointer_Button2: u32 = 0xfeea;
    pub const KEY_Pointer_Button3: u32 = 0xfeeb;
    pub const KEY_Pointer_Button4: u32 = 0xfeec;
    pub const KEY_Pointer_Button5: u32 = 0xfeed;
    pub const KEY_Pointer_DblClick_Dflt: u32 = 0xfeee;
    pub const KEY_Pointer_DblClick1: u32 = 0xfeef;
    pub const KEY_Pointer_DblClick2: u32 = 0xfef0;
    pub const KEY_Pointer_DblClick3: u32 = 0xfef1;
    pub const KEY_Pointer_DblClick4: u32 = 0xfef2;
    pub const KEY_Pointer_DblClick5: u32 = 0xfef3;
    pub const KEY_Pointer_Drag_Dflt: u32 = 0xfef4;
    pub const KEY_Pointer_Drag1: u32 = 0xfef5;
    pub const KEY_Pointer_Drag2: u32 = 0xfef6;
    pub const KEY_Pointer_Drag3: u32 = 0xfef7;
    pub const KEY_Pointer_Drag4: u32 = 0xfef8;
    pub const KEY_Pointer_Drag5: u32 = 0xfefd;
    pub const KEY_Pointer_EnableKeys: u32 = 0xfef9;
    pub const KEY_Pointer_Accelerate: u32 = 0xfefa;
    pub const KEY_Pointer_DfltBtnNext: u32 = 0xfefb;
    pub const KEY_Pointer_DfltBtnPrev: u32 = 0xfefc;
    pub const KEY_ch: u32 = 0xfea0;
    pub const KEY_Ch: u32 = 0xfea1;
    pub const KEY_CH: u32 = 0xfea2;
    pub const KEY_c_h: u32 = 0xfea3;
    pub const KEY_C_h: u32 = 0xfea4;
    pub const KEY_C_H: u32 = 0xfea5;
    pub const KEY_3270_Duplicate: u32 = 0xfd01;
    pub const KEY_3270_FieldMark: u32 = 0xfd02;
    pub const KEY_3270_Right2: u32 = 0xfd03;
    pub const KEY_3270_Left2: u32 = 0xfd04;
    pub const KEY_3270_BackTab: u32 = 0xfd05;
    pub const KEY_3270_EraseEOF: u32 = 0xfd06;
    pub const KEY_3270_EraseInput: u32 = 0xfd07;
    pub const KEY_3270_Reset: u32 = 0xfd08;
    pub const KEY_3270_Quit: u32 = 0xfd09;
    pub const KEY_3270_PA1: u32 = 0xfd0a;
    pub const KEY_3270_PA2: u32 = 0xfd0b;
    pub const KEY_3270_PA3: u32 = 0xfd0c;
    pub const KEY_3270_Test: u32 = 0xfd0d;
    pub const KEY_3270_Attn: u32 = 0xfd0e;
    pub const KEY_3270_CursorBlink: u32 = 0xfd0f;
    pub const KEY_3270_AltCursor: u32 = 0xfd10;
    pub const KEY_3270_KeyClick: u32 = 0xfd11;
    pub const KEY_3270_Jump: u32 = 0xfd12;
    pub const KEY_3270_Ident: u32 = 0xfd13;
    pub const KEY_3270_Rule: u32 = 0xfd14;
    pub const KEY_3270_Copy: u32 = 0xfd15;
    pub const KEY_3270_Play: u32 = 0xfd16;
    pub const KEY_3270_Setup: u32 = 0xfd17;
    pub const KEY_3270_Record: u32 = 0xfd18;
    pub const KEY_3270_ChangeScreen: u32 = 0xfd19;
    pub const KEY_3270_DeleteWord: u32 = 0xfd1a;
    pub const KEY_3270_ExSelect: u32 = 0xfd1b;
    pub const KEY_3270_CursorSelect: u32 = 0xfd1c;
    pub const KEY_3270_PrintScreen: u32 = 0xfd1d;
    pub const KEY_3270_Enter: u32 = 0xfd1e;
    pub const KEY_space: u32 = 0x0020;
    pub const KEY_exclam: u32 = 0x0021;
    pub const KEY_quotedbl: u32 = 0x0022;
    pub const KEY_numbersign: u32 = 0x0023;
    pub const KEY_dollar: u32 = 0x0024;
    pub const KEY_percent: u32 = 0x0025;
    pub const KEY_ampersand: u32 = 0x0026;
    pub const KEY_apostrophe: u32 = 0x0027;
    pub const KEY_quoteright: u32 = 0x0027;
    pub const KEY_parenleft: u32 = 0x0028;
    pub const KEY_parenright: u32 = 0x0029;
    pub const KEY_asterisk: u32 = 0x002a;
    pub const KEY_plus: u32 = 0x002b;
    pub const KEY_comma: u32 = 0x002c;
    pub const KEY_minus: u32 = 0x002d;
    pub const KEY_period: u32 = 0x002e;
    pub const KEY_slash: u32 = 0x002f;
    pub const KEY_0: u32 = 0x0030;
    pub const KEY_1: u32 = 0x0031;
    pub const KEY_2: u32 = 0x0032;
    pub const KEY_3: u32 = 0x0033;
    pub const KEY_4: u32 = 0x0034;
    pub const KEY_5: u32 = 0x0035;
    pub const KEY_6: u32 = 0x0036;
    pub const KEY_7: u32 = 0x0037;
    pub const KEY_8: u32 = 0x0038;
    pub const KEY_9: u32 = 0x0039;
    pub const KEY_colon: u32 = 0x003a;
    pub const KEY_semicolon: u32 = 0x003b;
    pub const KEY_less: u32 = 0x003c;
    pub const KEY_equal: u32 = 0x003d;
    pub const KEY_greater: u32 = 0x003e;
    pub const KEY_question: u32 = 0x003f;
    pub const KEY_at: u32 = 0x0040;
    pub const KEY_A: u32 = 0x0041;
    pub const KEY_B: u32 = 0x0042;
    pub const KEY_C: u32 = 0x0043;
    pub const KEY_D: u32 = 0x0044;
    pub const KEY_E: u32 = 0x0045;
    pub const KEY_F: u32 = 0x0046;
    pub const KEY_G: u32 = 0x0047;
    pub const KEY_H: u32 = 0x0048;
    pub const KEY_I: u32 = 0x0049;
    pub const KEY_J: u32 = 0x004a;
    pub const KEY_K: u32 = 0x004b;
    pub const KEY_L: u32 = 0x004c;
    pub const KEY_M: u32 = 0x004d;
    pub const KEY_N: u32 = 0x004e;
    pub const KEY_O: u32 = 0x004f;
    pub const KEY_P: u32 = 0x0050;
    pub const KEY_Q: u32 = 0x0051;
    pub const KEY_R: u32 = 0x0052;
    pub const KEY_S: u32 = 0x0053;
    pub const KEY_T: u32 = 0x0054;
    pub const KEY_U: u32 = 0x0055;
    pub const KEY_V: u32 = 0x0056;
    pub const KEY_W: u32 = 0x0057;
    pub const KEY_X: u32 = 0x0058;
    pub const KEY_Y: u32 = 0x0059;
    pub const KEY_Z: u32 = 0x005a;
    pub const KEY_bracketleft: u32 = 0x005b;
    pub const KEY_backslash: u32 = 0x005c;
    pub const KEY_bracketright: u32 = 0x005d;
    pub const KEY_asciicircum: u32 = 0x005e;
    pub const KEY_underscore: u32 = 0x005f;
    pub const KEY_grave: u32 = 0x0060;
    pub const KEY_quoteleft: u32 = 0x0060;
    pub const KEY_a: u32 = 0x0061;
    pub const KEY_b: u32 = 0x0062;
    pub const KEY_c: u32 = 0x0063;
    pub const KEY_d: u32 = 0x0064;
    pub const KEY_e: u32 = 0x0065;
    pub const KEY_f: u32 = 0x0066;
    pub const KEY_g: u32 = 0x0067;
    pub const KEY_h: u32 = 0x0068;
    pub const KEY_i: u32 = 0x0069;
    pub const KEY_j: u32 = 0x006a;
    pub const KEY_k: u32 = 0x006b;
    pub const KEY_l: u32 = 0x006c;
    pub const KEY_m: u32 = 0x006d;
    pub const KEY_n: u32 = 0x006e;
    pub const KEY_o: u32 = 0x006f;
    pub const KEY_p: u32 = 0x0070;
    pub const KEY_q: u32 = 0x0071;
    pub const KEY_r: u32 = 0x0072;
    pub const KEY_s: u32 = 0x0073;
    pub const KEY_t: u32 = 0x0074;
    pub const KEY_u: u32 = 0x0075;
    pub const KEY_v: u32 = 0x0076;
    pub const KEY_w: u32 = 0x0077;
    pub const KEY_x: u32 = 0x0078;
    pub const KEY_y: u32 = 0x0079;
    pub const KEY_z: u32 = 0x007a;
    pub const KEY_braceleft: u32 = 0x007b;
    pub const KEY_bar: u32 = 0x007c;
    pub const KEY_braceright: u32 = 0x007d;
    pub const KEY_asciitilde: u32 = 0x007e;
    pub const KEY_nobreakspace: u32 = 0x00a0;
    pub const KEY_exclamdown: u32 = 0x00a1;
    pub const KEY_cent: u32 = 0x00a2;
    pub const KEY_sterling: u32 = 0x00a3;
    pub const KEY_currency: u32 = 0x00a4;
    pub const KEY_yen: u32 = 0x00a5;
    pub const KEY_brokenbar: u32 = 0x00a6;
    pub const KEY_section: u32 = 0x00a7;
    pub const KEY_diaeresis: u32 = 0x00a8;
    pub const KEY_copyright: u32 = 0x00a9;
    pub const KEY_ordfeminine: u32 = 0x00aa;
    pub const KEY_guillemotleft: u32 = 0x00ab;
    pub const KEY_guillemetleft: u32 = 0x00ab;
    pub const KEY_notsign: u32 = 0x00ac;
    pub const KEY_hyphen: u32 = 0x00ad;
    pub const KEY_registered: u32 = 0x00ae;
    pub const KEY_macron: u32 = 0x00af;
    pub const KEY_degree: u32 = 0x00b0;
    pub const KEY_plusminus: u32 = 0x00b1;
    pub const KEY_twosuperior: u32 = 0x00b2;
    pub const KEY_threesuperior: u32 = 0x00b3;
    pub const KEY_acute: u32 = 0x00b4;
    pub const KEY_mu: u32 = 0x00b5;
    pub const KEY_paragraph: u32 = 0x00b6;
    pub const KEY_periodcentered: u32 = 0x00b7;
    pub const KEY_cedilla: u32 = 0x00b8;
    pub const KEY_onesuperior: u32 = 0x00b9;
    pub const KEY_masculine: u32 = 0x00ba;
    pub const KEY_ordmasculine: u32 = 0x00ba;
    pub const KEY_guillemotright: u32 = 0x00bb;
    pub const KEY_guillemetright: u32 = 0x00bb;
    pub const KEY_onequarter: u32 = 0x00bc;
    pub const KEY_onehalf: u32 = 0x00bd;
    pub const KEY_threequarters: u32 = 0x00be;
    pub const KEY_questiondown: u32 = 0x00bf;
    pub const KEY_Agrave: u32 = 0x00c0;
    pub const KEY_Aacute: u32 = 0x00c1;
    pub const KEY_Acircumflex: u32 = 0x00c2;
    pub const KEY_Atilde: u32 = 0x00c3;
    pub const KEY_Adiaeresis: u32 = 0x00c4;
    pub const KEY_Aring: u32 = 0x00c5;
    pub const KEY_AE: u32 = 0x00c6;
    pub const KEY_Ccedilla: u32 = 0x00c7;
    pub const KEY_Egrave: u32 = 0x00c8;
    pub const KEY_Eacute: u32 = 0x00c9;
    pub const KEY_Ecircumflex: u32 = 0x00ca;
    pub const KEY_Ediaeresis: u32 = 0x00cb;
    pub const KEY_Igrave: u32 = 0x00cc;
    pub const KEY_Iacute: u32 = 0x00cd;
    pub const KEY_Icircumflex: u32 = 0x00ce;
    pub const KEY_Idiaeresis: u32 = 0x00cf;
    pub const KEY_ETH: u32 = 0x00d0;
    pub const KEY_Eth: u32 = 0x00d0;
    pub const KEY_Ntilde: u32 = 0x00d1;
    pub const KEY_Ograve: u32 = 0x00d2;
    pub const KEY_Oacute: u32 = 0x00d3;
    pub const KEY_Ocircumflex: u32 = 0x00d4;
    pub const KEY_Otilde: u32 = 0x00d5;
    pub const KEY_Odiaeresis: u32 = 0x00d6;
    pub const KEY_multiply: u32 = 0x00d7;
    pub const KEY_Oslash: u32 = 0x00d8;
    pub const KEY_Ooblique: u32 = 0x00d8;
    pub const KEY_Ugrave: u32 = 0x00d9;
    pub const KEY_Uacute: u32 = 0x00da;
    pub const KEY_Ucircumflex: u32 = 0x00db;
    pub const KEY_Udiaeresis: u32 = 0x00dc;
    pub const KEY_Yacute: u32 = 0x00dd;
    pub const KEY_THORN: u32 = 0x00de;
    pub const KEY_Thorn: u32 = 0x00de;
    pub const KEY_ssharp: u32 = 0x00df;
    pub const KEY_agrave: u32 = 0x00e0;
    pub const KEY_aacute: u32 = 0x00e1;
    pub const KEY_acircumflex: u32 = 0x00e2;
    pub const KEY_atilde: u32 = 0x00e3;
    pub const KEY_adiaeresis: u32 = 0x00e4;
    pub const KEY_aring: u32 = 0x00e5;
    pub const KEY_ae: u32 = 0x00e6;
    pub const KEY_ccedilla: u32 = 0x00e7;
    pub const KEY_egrave: u32 = 0x00e8;
    pub const KEY_eacute: u32 = 0x00e9;
    pub const KEY_ecircumflex: u32 = 0x00ea;
    pub const KEY_ediaeresis: u32 = 0x00eb;
    pub const KEY_igrave: u32 = 0x00ec;
    pub const KEY_iacute: u32 = 0x00ed;
    pub const KEY_icircumflex: u32 = 0x00ee;
    pub const KEY_idiaeresis: u32 = 0x00ef;
    pub const KEY_eth: u32 = 0x00f0;
    pub const KEY_ntilde: u32 = 0x00f1;
    pub const KEY_ograve: u32 = 0x00f2;
    pub const KEY_oacute: u32 = 0x00f3;
    pub const KEY_ocircumflex: u32 = 0x00f4;
    pub const KEY_otilde: u32 = 0x00f5;
    pub const KEY_odiaeresis: u32 = 0x00f6;
    pub const KEY_division: u32 = 0x00f7;
    pub const KEY_oslash: u32 = 0x00f8;
    pub const KEY_ooblique: u32 = 0x00f8;
    pub const KEY_ugrave: u32 = 0x00f9;
    pub const KEY_uacute: u32 = 0x00fa;
    pub const KEY_ucircumflex: u32 = 0x00fb;
    pub const KEY_udiaeresis: u32 = 0x00fc;
    pub const KEY_yacute: u32 = 0x00fd;
    pub const KEY_thorn: u32 = 0x00fe;
    pub const KEY_ydiaeresis: u32 = 0x00ff;
    pub const KEY_Aogonek: u32 = 0x01a1;
    pub const KEY_breve: u32 = 0x01a2;
    pub const KEY_Lstroke: u32 = 0x01a3;
    pub const KEY_Lcaron: u32 = 0x01a5;
    pub const KEY_Sacute: u32 = 0x01a6;
    pub const KEY_Scaron: u32 = 0x01a9;
    pub const KEY_Scedilla: u32 = 0x01aa;
    pub const KEY_Tcaron: u32 = 0x01ab;
    pub const KEY_Zacute: u32 = 0x01ac;
    pub const KEY_Zcaron: u32 = 0x01ae;
    pub const KEY_Zabovedot: u32 = 0x01af;
    pub const KEY_aogonek: u32 = 0x01b1;
    pub const KEY_ogonek: u32 = 0x01b2;
    pub const KEY_lstroke: u32 = 0x01b3;
    pub const KEY_lcaron: u32 = 0x01b5;
    pub const KEY_sacute: u32 = 0x01b6;
    pub const KEY_caron: u32 = 0x01b7;
    pub const KEY_scaron: u32 = 0x01b9;
    pub const KEY_scedilla: u32 = 0x01ba;
    pub const KEY_tcaron: u32 = 0x01bb;
    pub const KEY_zacute: u32 = 0x01bc;
    pub const KEY_doubleacute: u32 = 0x01bd;
    pub const KEY_zcaron: u32 = 0x01be;
    pub const KEY_zabovedot: u32 = 0x01bf;
    pub const KEY_Racute: u32 = 0x01c0;
    pub const KEY_Abreve: u32 = 0x01c3;
    pub const KEY_Lacute: u32 = 0x01c5;
    pub const KEY_Cacute: u32 = 0x01c6;
    pub const KEY_Ccaron: u32 = 0x01c8;
    pub const KEY_Eogonek: u32 = 0x01ca;
    pub const KEY_Ecaron: u32 = 0x01cc;
    pub const KEY_Dcaron: u32 = 0x01cf;
    pub const KEY_Dstroke: u32 = 0x01d0;
    pub const KEY_Nacute: u32 = 0x01d1;
    pub const KEY_Ncaron: u32 = 0x01d2;
    pub const KEY_Odoubleacute: u32 = 0x01d5;
    pub const KEY_Rcaron: u32 = 0x01d8;
    pub const KEY_Uring: u32 = 0x01d9;
    pub const KEY_Udoubleacute: u32 = 0x01db;
    pub const KEY_Tcedilla: u32 = 0x01de;
    pub const KEY_racute: u32 = 0x01e0;
    pub const KEY_abreve: u32 = 0x01e3;
    pub const KEY_lacute: u32 = 0x01e5;
    pub const KEY_cacute: u32 = 0x01e6;
    pub const KEY_ccaron: u32 = 0x01e8;
    pub const KEY_eogonek: u32 = 0x01ea;
    pub const KEY_ecaron: u32 = 0x01ec;
    pub const KEY_dcaron: u32 = 0x01ef;
    pub const KEY_dstroke: u32 = 0x01f0;
    pub const KEY_nacute: u32 = 0x01f1;
    pub const KEY_ncaron: u32 = 0x01f2;
    pub const KEY_odoubleacute: u32 = 0x01f5;
    pub const KEY_rcaron: u32 = 0x01f8;
    pub const KEY_uring: u32 = 0x01f9;
    pub const KEY_udoubleacute: u32 = 0x01fb;
    pub const KEY_tcedilla: u32 = 0x01fe;
    pub const KEY_abovedot: u32 = 0x01ff;
    pub const KEY_Hstroke: u32 = 0x02a1;
    pub const KEY_Hcircumflex: u32 = 0x02a6;
    pub const KEY_Iabovedot: u32 = 0x02a9;
    pub const KEY_Gbreve: u32 = 0x02ab;
    pub const KEY_Jcircumflex: u32 = 0x02ac;
    pub const KEY_hstroke: u32 = 0x02b1;
    pub const KEY_hcircumflex: u32 = 0x02b6;
    pub const KEY_idotless: u32 = 0x02b9;
    pub const KEY_gbreve: u32 = 0x02bb;
    pub const KEY_jcircumflex: u32 = 0x02bc;
    pub const KEY_Cabovedot: u32 = 0x02c5;
    pub const KEY_Ccircumflex: u32 = 0x02c6;
    pub const KEY_Gabovedot: u32 = 0x02d5;
    pub const KEY_Gcircumflex: u32 = 0x02d8;
    pub const KEY_Ubreve: u32 = 0x02dd;
    pub const KEY_Scircumflex: u32 = 0x02de;
    pub const KEY_cabovedot: u32 = 0x02e5;
    pub const KEY_ccircumflex: u32 = 0x02e6;
    pub const KEY_gabovedot: u32 = 0x02f5;
    pub const KEY_gcircumflex: u32 = 0x02f8;
    pub const KEY_ubreve: u32 = 0x02fd;
    pub const KEY_scircumflex: u32 = 0x02fe;
    pub const KEY_kra: u32 = 0x03a2;
    pub const KEY_kappa: u32 = 0x03a2;
    pub const KEY_Rcedilla: u32 = 0x03a3;
    pub const KEY_Itilde: u32 = 0x03a5;
    pub const KEY_Lcedilla: u32 = 0x03a6;
    pub const KEY_Emacron: u32 = 0x03aa;
    pub const KEY_Gcedilla: u32 = 0x03ab;
    pub const KEY_Tslash: u32 = 0x03ac;
    pub const KEY_rcedilla: u32 = 0x03b3;
    pub const KEY_itilde: u32 = 0x03b5;
    pub const KEY_lcedilla: u32 = 0x03b6;
    pub const KEY_emacron: u32 = 0x03ba;
    pub const KEY_gcedilla: u32 = 0x03bb;
    pub const KEY_tslash: u32 = 0x03bc;
    pub const KEY_ENG: u32 = 0x03bd;
    pub const KEY_eng: u32 = 0x03bf;
    pub const KEY_Amacron: u32 = 0x03c0;
    pub const KEY_Iogonek: u32 = 0x03c7;
    pub const KEY_Eabovedot: u32 = 0x03cc;
    pub const KEY_Imacron: u32 = 0x03cf;
    pub const KEY_Ncedilla: u32 = 0x03d1;
    pub const KEY_Omacron: u32 = 0x03d2;
    pub const KEY_Kcedilla: u32 = 0x03d3;
    pub const KEY_Uogonek: u32 = 0x03d9;
    pub const KEY_Utilde: u32 = 0x03dd;
    pub const KEY_Umacron: u32 = 0x03de;
    pub const KEY_amacron: u32 = 0x03e0;
    pub const KEY_iogonek: u32 = 0x03e7;
    pub const KEY_eabovedot: u32 = 0x03ec;
    pub const KEY_imacron: u32 = 0x03ef;
    pub const KEY_ncedilla: u32 = 0x03f1;
    pub const KEY_omacron: u32 = 0x03f2;
    pub const KEY_kcedilla: u32 = 0x03f3;
    pub const KEY_uogonek: u32 = 0x03f9;
    pub const KEY_utilde: u32 = 0x03fd;
    pub const KEY_umacron: u32 = 0x03fe;
    pub const KEY_Wcircumflex: u32 = 0x1000174;
    pub const KEY_wcircumflex: u32 = 0x1000175;
    pub const KEY_Ycircumflex: u32 = 0x1000176;
    pub const KEY_ycircumflex: u32 = 0x1000177;
    pub const KEY_Babovedot: u32 = 0x1001e02;
    pub const KEY_babovedot: u32 = 0x1001e03;
    pub const KEY_Dabovedot: u32 = 0x1001e0a;
    pub const KEY_dabovedot: u32 = 0x1001e0b;
    pub const KEY_Fabovedot: u32 = 0x1001e1e;
    pub const KEY_fabovedot: u32 = 0x1001e1f;
    pub const KEY_Mabovedot: u32 = 0x1001e40;
    pub const KEY_mabovedot: u32 = 0x1001e41;
    pub const KEY_Pabovedot: u32 = 0x1001e56;
    pub const KEY_pabovedot: u32 = 0x1001e57;
    pub const KEY_Sabovedot: u32 = 0x1001e60;
    pub const KEY_sabovedot: u32 = 0x1001e61;
    pub const KEY_Tabovedot: u32 = 0x1001e6a;
    pub const KEY_tabovedot: u32 = 0x1001e6b;
    pub const KEY_Wgrave: u32 = 0x1001e80;
    pub const KEY_wgrave: u32 = 0x1001e81;
    pub const KEY_Wacute: u32 = 0x1001e82;
    pub const KEY_wacute: u32 = 0x1001e83;
    pub const KEY_Wdiaeresis: u32 = 0x1001e84;
    pub const KEY_wdiaeresis: u32 = 0x1001e85;
    pub const KEY_Ygrave: u32 = 0x1001ef2;
    pub const KEY_ygrave: u32 = 0x1001ef3;
    pub const KEY_OE: u32 = 0x13bc;
    pub const KEY_oe: u32 = 0x13bd;
    pub const KEY_Ydiaeresis: u32 = 0x13be;
    pub const KEY_overline: u32 = 0x047e;
    pub const KEY_kana_fullstop: u32 = 0x04a1;
    pub const KEY_kana_openingbracket: u32 = 0x04a2;
    pub const KEY_kana_closingbracket: u32 = 0x04a3;
    pub const KEY_kana_comma: u32 = 0x04a4;
    pub const KEY_kana_conjunctive: u32 = 0x04a5;
    pub const KEY_kana_middledot: u32 = 0x04a5;
    pub const KEY_kana_WO: u32 = 0x04a6;
    pub const KEY_kana_a: u32 = 0x04a7;
    pub const KEY_kana_i: u32 = 0x04a8;
    pub const KEY_kana_u: u32 = 0x04a9;
    pub const KEY_kana_e: u32 = 0x04aa;
    pub const KEY_kana_o: u32 = 0x04ab;
    pub const KEY_kana_ya: u32 = 0x04ac;
    pub const KEY_kana_yu: u32 = 0x04ad;
    pub const KEY_kana_yo: u32 = 0x04ae;
    pub const KEY_kana_tsu: u32 = 0x04af;
    pub const KEY_kana_tu: u32 = 0x04af;
    pub const KEY_prolongedsound: u32 = 0x04b0;
    pub const KEY_kana_A: u32 = 0x04b1;
    pub const KEY_kana_I: u32 = 0x04b2;
    pub const KEY_kana_U: u32 = 0x04b3;
    pub const KEY_kana_E: u32 = 0x04b4;
    pub const KEY_kana_O: u32 = 0x04b5;
    pub const KEY_kana_KA: u32 = 0x04b6;
    pub const KEY_kana_KI: u32 = 0x04b7;
    pub const KEY_kana_KU: u32 = 0x04b8;
    pub const KEY_kana_KE: u32 = 0x04b9;
    pub const KEY_kana_KO: u32 = 0x04ba;
    pub const KEY_kana_SA: u32 = 0x04bb;
    pub const KEY_kana_SHI: u32 = 0x04bc;
    pub const KEY_kana_SU: u32 = 0x04bd;
    pub const KEY_kana_SE: u32 = 0x04be;
    pub const KEY_kana_SO: u32 = 0x04bf;
    pub const KEY_kana_TA: u32 = 0x04c0;
    pub const KEY_kana_CHI: u32 = 0x04c1;
    pub const KEY_kana_TI: u32 = 0x04c1;
    pub const KEY_kana_TSU: u32 = 0x04c2;
    pub const KEY_kana_TU: u32 = 0x04c2;
    pub const KEY_kana_TE: u32 = 0x04c3;
    pub const KEY_kana_TO: u32 = 0x04c4;
    pub const KEY_kana_NA: u32 = 0x04c5;
    pub const KEY_kana_NI: u32 = 0x04c6;
    pub const KEY_kana_NU: u32 = 0x04c7;
    pub const KEY_kana_NE: u32 = 0x04c8;
    pub const KEY_kana_NO: u32 = 0x04c9;
    pub const KEY_kana_HA: u32 = 0x04ca;
    pub const KEY_kana_HI: u32 = 0x04cb;
    pub const KEY_kana_FU: u32 = 0x04cc;
    pub const KEY_kana_HU: u32 = 0x04cc;
    pub const KEY_kana_HE: u32 = 0x04cd;
    pub const KEY_kana_HO: u32 = 0x04ce;
    pub const KEY_kana_MA: u32 = 0x04cf;
    pub const KEY_kana_MI: u32 = 0x04d0;
    pub const KEY_kana_MU: u32 = 0x04d1;
    pub const KEY_kana_ME: u32 = 0x04d2;
    pub const KEY_kana_MO: u32 = 0x04d3;
    pub const KEY_kana_YA: u32 = 0x04d4;
    pub const KEY_kana_YU: u32 = 0x04d5;
    pub const KEY_kana_YO: u32 = 0x04d6;
    pub const KEY_kana_RA: u32 = 0x04d7;
    pub const KEY_kana_RI: u32 = 0x04d8;
    pub const KEY_kana_RU: u32 = 0x04d9;
    pub const KEY_kana_RE: u32 = 0x04da;
    pub const KEY_kana_RO: u32 = 0x04db;
    pub const KEY_kana_WA: u32 = 0x04dc;
    pub const KEY_kana_N: u32 = 0x04dd;
    pub const KEY_voicedsound: u32 = 0x04de;
    pub const KEY_semivoicedsound: u32 = 0x04df;
    pub const KEY_kana_switch: u32 = 0xff7e;
    pub const KEY_Farsi_0: u32 = 0x10006f0;
    pub const KEY_Farsi_1: u32 = 0x10006f1;
    pub const KEY_Farsi_2: u32 = 0x10006f2;
    pub const KEY_Farsi_3: u32 = 0x10006f3;
    pub const KEY_Farsi_4: u32 = 0x10006f4;
    pub const KEY_Farsi_5: u32 = 0x10006f5;
    pub const KEY_Farsi_6: u32 = 0x10006f6;
    pub const KEY_Farsi_7: u32 = 0x10006f7;
    pub const KEY_Farsi_8: u32 = 0x10006f8;
    pub const KEY_Farsi_9: u32 = 0x10006f9;
    pub const KEY_Arabic_percent: u32 = 0x100066a;
    pub const KEY_Arabic_superscript_alef: u32 = 0x1000670;
    pub const KEY_Arabic_tteh: u32 = 0x1000679;
    pub const KEY_Arabic_peh: u32 = 0x100067e;
    pub const KEY_Arabic_tcheh: u32 = 0x1000686;
    pub const KEY_Arabic_ddal: u32 = 0x1000688;
    pub const KEY_Arabic_rreh: u32 = 0x1000691;
    pub const KEY_Arabic_comma: u32 = 0x05ac;
    pub const KEY_Arabic_fullstop: u32 = 0x10006d4;
    pub const KEY_Arabic_0: u32 = 0x1000660;
    pub const KEY_Arabic_1: u32 = 0x1000661;
    pub const KEY_Arabic_2: u32 = 0x1000662;
    pub const KEY_Arabic_3: u32 = 0x1000663;
    pub const KEY_Arabic_4: u32 = 0x1000664;
    pub const KEY_Arabic_5: u32 = 0x1000665;
    pub const KEY_Arabic_6: u32 = 0x1000666;
    pub const KEY_Arabic_7: u32 = 0x1000667;
    pub const KEY_Arabic_8: u32 = 0x1000668;
    pub const KEY_Arabic_9: u32 = 0x1000669;
    pub const KEY_Arabic_semicolon: u32 = 0x05bb;
    pub const KEY_Arabic_question_mark: u32 = 0x05bf;
    pub const KEY_Arabic_hamza: u32 = 0x05c1;
    pub const KEY_Arabic_maddaonalef: u32 = 0x05c2;
    pub const KEY_Arabic_hamzaonalef: u32 = 0x05c3;
    pub const KEY_Arabic_hamzaonwaw: u32 = 0x05c4;
    pub const KEY_Arabic_hamzaunderalef: u32 = 0x05c5;
    pub const KEY_Arabic_hamzaonyeh: u32 = 0x05c6;
    pub const KEY_Arabic_alef: u32 = 0x05c7;
    pub const KEY_Arabic_beh: u32 = 0x05c8;
    pub const KEY_Arabic_tehmarbuta: u32 = 0x05c9;
    pub const KEY_Arabic_teh: u32 = 0x05ca;
    pub const KEY_Arabic_theh: u32 = 0x05cb;
    pub const KEY_Arabic_jeem: u32 = 0x05cc;
    pub const KEY_Arabic_hah: u32 = 0x05cd;
    pub const KEY_Arabic_khah: u32 = 0x05ce;
    pub const KEY_Arabic_dal: u32 = 0x05cf;
    pub const KEY_Arabic_thal: u32 = 0x05d0;
    pub const KEY_Arabic_ra: u32 = 0x05d1;
    pub const KEY_Arabic_zain: u32 = 0x05d2;
    pub const KEY_Arabic_seen: u32 = 0x05d3;
    pub const KEY_Arabic_sheen: u32 = 0x05d4;
    pub const KEY_Arabic_sad: u32 = 0x05d5;
    pub const KEY_Arabic_dad: u32 = 0x05d6;
    pub const KEY_Arabic_tah: u32 = 0x05d7;
    pub const KEY_Arabic_zah: u32 = 0x05d8;
    pub const KEY_Arabic_ain: u32 = 0x05d9;
    pub const KEY_Arabic_ghain: u32 = 0x05da;
    pub const KEY_Arabic_tatweel: u32 = 0x05e0;
    pub const KEY_Arabic_feh: u32 = 0x05e1;
    pub const KEY_Arabic_qaf: u32 = 0x05e2;
    pub const KEY_Arabic_kaf: u32 = 0x05e3;
    pub const KEY_Arabic_lam: u32 = 0x05e4;
    pub const KEY_Arabic_meem: u32 = 0x05e5;
    pub const KEY_Arabic_noon: u32 = 0x05e6;
    pub const KEY_Arabic_ha: u32 = 0x05e7;
    pub const KEY_Arabic_heh: u32 = 0x05e7;
    pub const KEY_Arabic_waw: u32 = 0x05e8;
    pub const KEY_Arabic_alefmaksura: u32 = 0x05e9;
    pub const KEY_Arabic_yeh: u32 = 0x05ea;
    pub const KEY_Arabic_fathatan: u32 = 0x05eb;
    pub const KEY_Arabic_dammatan: u32 = 0x05ec;
    pub const KEY_Arabic_kasratan: u32 = 0x05ed;
    pub const KEY_Arabic_fatha: u32 = 0x05ee;
    pub const KEY_Arabic_damma: u32 = 0x05ef;
    pub const KEY_Arabic_kasra: u32 = 0x05f0;
    pub const KEY_Arabic_shadda: u32 = 0x05f1;
    pub const KEY_Arabic_sukun: u32 = 0x05f2;
    pub const KEY_Arabic_madda_above: u32 = 0x1000653;
    pub const KEY_Arabic_hamza_above: u32 = 0x1000654;
    pub const KEY_Arabic_hamza_below: u32 = 0x1000655;
    pub const KEY_Arabic_jeh: u32 = 0x1000698;
    pub const KEY_Arabic_veh: u32 = 0x10006a4;
    pub const KEY_Arabic_keheh: u32 = 0x10006a9;
    pub const KEY_Arabic_gaf: u32 = 0x10006af;
    pub const KEY_Arabic_noon_ghunna: u32 = 0x10006ba;
    pub const KEY_Arabic_heh_doachashmee: u32 = 0x10006be;
    pub const KEY_Farsi_yeh: u32 = 0x10006cc;
    pub const KEY_Arabic_farsi_yeh: u32 = 0x10006cc;
    pub const KEY_Arabic_yeh_baree: u32 = 0x10006d2;
    pub const KEY_Arabic_heh_goal: u32 = 0x10006c1;
    pub const KEY_Arabic_switch: u32 = 0xff7e;
    pub const KEY_Cyrillic_GHE_bar: u32 = 0x1000492;
    pub const KEY_Cyrillic_ghe_bar: u32 = 0x1000493;
    pub const KEY_Cyrillic_ZHE_descender: u32 = 0x1000496;
    pub const KEY_Cyrillic_zhe_descender: u32 = 0x1000497;
    pub const KEY_Cyrillic_KA_descender: u32 = 0x100049a;
    pub const KEY_Cyrillic_ka_descender: u32 = 0x100049b;
    pub const KEY_Cyrillic_KA_vertstroke: u32 = 0x100049c;
    pub const KEY_Cyrillic_ka_vertstroke: u32 = 0x100049d;
    pub const KEY_Cyrillic_EN_descender: u32 = 0x10004a2;
    pub const KEY_Cyrillic_en_descender: u32 = 0x10004a3;
    pub const KEY_Cyrillic_U_straight: u32 = 0x10004ae;
    pub const KEY_Cyrillic_u_straight: u32 = 0x10004af;
    pub const KEY_Cyrillic_U_straight_bar: u32 = 0x10004b0;
    pub const KEY_Cyrillic_u_straight_bar: u32 = 0x10004b1;
    pub const KEY_Cyrillic_HA_descender: u32 = 0x10004b2;
    pub const KEY_Cyrillic_ha_descender: u32 = 0x10004b3;
    pub const KEY_Cyrillic_CHE_descender: u32 = 0x10004b6;
    pub const KEY_Cyrillic_che_descender: u32 = 0x10004b7;
    pub const KEY_Cyrillic_CHE_vertstroke: u32 = 0x10004b8;
    pub const KEY_Cyrillic_che_vertstroke: u32 = 0x10004b9;
    pub const KEY_Cyrillic_SHHA: u32 = 0x10004ba;
    pub const KEY_Cyrillic_shha: u32 = 0x10004bb;
    pub const KEY_Cyrillic_SCHWA: u32 = 0x10004d8;
    pub const KEY_Cyrillic_schwa: u32 = 0x10004d9;
    pub const KEY_Cyrillic_I_macron: u32 = 0x10004e2;
    pub const KEY_Cyrillic_i_macron: u32 = 0x10004e3;
    pub const KEY_Cyrillic_O_bar: u32 = 0x10004e8;
    pub const KEY_Cyrillic_o_bar: u32 = 0x10004e9;
    pub const KEY_Cyrillic_U_macron: u32 = 0x10004ee;
    pub const KEY_Cyrillic_u_macron: u32 = 0x10004ef;
    pub const KEY_Serbian_dje: u32 = 0x06a1;
    pub const KEY_Macedonia_gje: u32 = 0x06a2;
    pub const KEY_Cyrillic_io: u32 = 0x06a3;
    pub const KEY_Ukrainian_ie: u32 = 0x06a4;
    pub const KEY_Ukranian_je: u32 = 0x06a4;
    pub const KEY_Macedonia_dse: u32 = 0x06a5;
    pub const KEY_Ukrainian_i: u32 = 0x06a6;
    pub const KEY_Ukranian_i: u32 = 0x06a6;
    pub const KEY_Ukrainian_yi: u32 = 0x06a7;
    pub const KEY_Ukranian_yi: u32 = 0x06a7;
    pub const KEY_Cyrillic_je: u32 = 0x06a8;
    pub const KEY_Serbian_je: u32 = 0x06a8;
    pub const KEY_Cyrillic_lje: u32 = 0x06a9;
    pub const KEY_Serbian_lje: u32 = 0x06a9;
    pub const KEY_Cyrillic_nje: u32 = 0x06aa;
    pub const KEY_Serbian_nje: u32 = 0x06aa;
    pub const KEY_Serbian_tshe: u32 = 0x06ab;
    pub const KEY_Macedonia_kje: u32 = 0x06ac;
    pub const KEY_Ukrainian_ghe_with_upturn: u32 = 0x06ad;
    pub const KEY_Byelorussian_shortu: u32 = 0x06ae;
    pub const KEY_Cyrillic_dzhe: u32 = 0x06af;
    pub const KEY_Serbian_dze: u32 = 0x06af;
    pub const KEY_numerosign: u32 = 0x06b0;
    pub const KEY_Serbian_DJE: u32 = 0x06b1;
    pub const KEY_Macedonia_GJE: u32 = 0x06b2;
    pub const KEY_Cyrillic_IO: u32 = 0x06b3;
    pub const KEY_Ukrainian_IE: u32 = 0x06b4;
    pub const KEY_Ukranian_JE: u32 = 0x06b4;
    pub const KEY_Macedonia_DSE: u32 = 0x06b5;
    pub const KEY_Ukrainian_I: u32 = 0x06b6;
    pub const KEY_Ukranian_I: u32 = 0x06b6;
    pub const KEY_Ukrainian_YI: u32 = 0x06b7;
    pub const KEY_Ukranian_YI: u32 = 0x06b7;
    pub const KEY_Cyrillic_JE: u32 = 0x06b8;
    pub const KEY_Serbian_JE: u32 = 0x06b8;
    pub const KEY_Cyrillic_LJE: u32 = 0x06b9;
    pub const KEY_Serbian_LJE: u32 = 0x06b9;
    pub const KEY_Cyrillic_NJE: u32 = 0x06ba;
    pub const KEY_Serbian_NJE: u32 = 0x06ba;
    pub const KEY_Serbian_TSHE: u32 = 0x06bb;
    pub const KEY_Macedonia_KJE: u32 = 0x06bc;
    pub const KEY_Ukrainian_GHE_WITH_UPTURN: u32 = 0x06bd;
    pub const KEY_Byelorussian_SHORTU: u32 = 0x06be;
    pub const KEY_Cyrillic_DZHE: u32 = 0x06bf;
    pub const KEY_Serbian_DZE: u32 = 0x06bf;
    pub const KEY_Cyrillic_yu: u32 = 0x06c0;
    pub const KEY_Cyrillic_a: u32 = 0x06c1;
    pub const KEY_Cyrillic_be: u32 = 0x06c2;
    pub const KEY_Cyrillic_tse: u32 = 0x06c3;
    pub const KEY_Cyrillic_de: u32 = 0x06c4;
    pub const KEY_Cyrillic_ie: u32 = 0x06c5;
    pub const KEY_Cyrillic_ef: u32 = 0x06c6;
    pub const KEY_Cyrillic_ghe: u32 = 0x06c7;
    pub const KEY_Cyrillic_ha: u32 = 0x06c8;
    pub const KEY_Cyrillic_i: u32 = 0x06c9;
    pub const KEY_Cyrillic_shorti: u32 = 0x06ca;
    pub const KEY_Cyrillic_ka: u32 = 0x06cb;
    pub const KEY_Cyrillic_el: u32 = 0x06cc;
    pub const KEY_Cyrillic_em: u32 = 0x06cd;
    pub const KEY_Cyrillic_en: u32 = 0x06ce;
    pub const KEY_Cyrillic_o: u32 = 0x06cf;
    pub const KEY_Cyrillic_pe: u32 = 0x06d0;
    pub const KEY_Cyrillic_ya: u32 = 0x06d1;
    pub const KEY_Cyrillic_er: u32 = 0x06d2;
    pub const KEY_Cyrillic_es: u32 = 0x06d3;
    pub const KEY_Cyrillic_te: u32 = 0x06d4;
    pub const KEY_Cyrillic_u: u32 = 0x06d5;
    pub const KEY_Cyrillic_zhe: u32 = 0x06d6;
    pub const KEY_Cyrillic_ve: u32 = 0x06d7;
    pub const KEY_Cyrillic_softsign: u32 = 0x06d8;
    pub const KEY_Cyrillic_yeru: u32 = 0x06d9;
    pub const KEY_Cyrillic_ze: u32 = 0x06da;
    pub const KEY_Cyrillic_sha: u32 = 0x06db;
    pub const KEY_Cyrillic_e: u32 = 0x06dc;
    pub const KEY_Cyrillic_shcha: u32 = 0x06dd;
    pub const KEY_Cyrillic_che: u32 = 0x06de;
    pub const KEY_Cyrillic_hardsign: u32 = 0x06df;
    pub const KEY_Cyrillic_YU: u32 = 0x06e0;
    pub const KEY_Cyrillic_A: u32 = 0x06e1;
    pub const KEY_Cyrillic_BE: u32 = 0x06e2;
    pub const KEY_Cyrillic_TSE: u32 = 0x06e3;
    pub const KEY_Cyrillic_DE: u32 = 0x06e4;
    pub const KEY_Cyrillic_IE: u32 = 0x06e5;
    pub const KEY_Cyrillic_EF: u32 = 0x06e6;
    pub const KEY_Cyrillic_GHE: u32 = 0x06e7;
    pub const KEY_Cyrillic_HA: u32 = 0x06e8;
    pub const KEY_Cyrillic_I: u32 = 0x06e9;
    pub const KEY_Cyrillic_SHORTI: u32 = 0x06ea;
    pub const KEY_Cyrillic_KA: u32 = 0x06eb;
    pub const KEY_Cyrillic_EL: u32 = 0x06ec;
    pub const KEY_Cyrillic_EM: u32 = 0x06ed;
    pub const KEY_Cyrillic_EN: u32 = 0x06ee;
    pub const KEY_Cyrillic_O: u32 = 0x06ef;
    pub const KEY_Cyrillic_PE: u32 = 0x06f0;
    pub const KEY_Cyrillic_YA: u32 = 0x06f1;
    pub const KEY_Cyrillic_ER: u32 = 0x06f2;
    pub const KEY_Cyrillic_ES: u32 = 0x06f3;
    pub const KEY_Cyrillic_TE: u32 = 0x06f4;
    pub const KEY_Cyrillic_U: u32 = 0x06f5;
    pub const KEY_Cyrillic_ZHE: u32 = 0x06f6;
    pub const KEY_Cyrillic_VE: u32 = 0x06f7;
    pub const KEY_Cyrillic_SOFTSIGN: u32 = 0x06f8;
    pub const KEY_Cyrillic_YERU: u32 = 0x06f9;
    pub const KEY_Cyrillic_ZE: u32 = 0x06fa;
    pub const KEY_Cyrillic_SHA: u32 = 0x06fb;
    pub const KEY_Cyrillic_E: u32 = 0x06fc;
    pub const KEY_Cyrillic_SHCHA: u32 = 0x06fd;
    pub const KEY_Cyrillic_CHE: u32 = 0x06fe;
    pub const KEY_Cyrillic_HARDSIGN: u32 = 0x06ff;
    pub const KEY_Greek_ALPHAaccent: u32 = 0x07a1;
    pub const KEY_Greek_EPSILONaccent: u32 = 0x07a2;
    pub const KEY_Greek_ETAaccent: u32 = 0x07a3;
    pub const KEY_Greek_IOTAaccent: u32 = 0x07a4;
    pub const KEY_Greek_IOTAdieresis: u32 = 0x07a5;
    pub const KEY_Greek_IOTAdiaeresis: u32 = 0x07a5;
    pub const KEY_Greek_OMICRONaccent: u32 = 0x07a7;
    pub const KEY_Greek_UPSILONaccent: u32 = 0x07a8;
    pub const KEY_Greek_UPSILONdieresis: u32 = 0x07a9;
    pub const KEY_Greek_OMEGAaccent: u32 = 0x07ab;
    pub const KEY_Greek_accentdieresis: u32 = 0x07ae;
    pub const KEY_Greek_horizbar: u32 = 0x07af;
    pub const KEY_Greek_alphaaccent: u32 = 0x07b1;
    pub const KEY_Greek_epsilonaccent: u32 = 0x07b2;
    pub const KEY_Greek_etaaccent: u32 = 0x07b3;
    pub const KEY_Greek_iotaaccent: u32 = 0x07b4;
    pub const KEY_Greek_iotadieresis: u32 = 0x07b5;
    pub const KEY_Greek_iotaaccentdieresis: u32 = 0x07b6;
    pub const KEY_Greek_omicronaccent: u32 = 0x07b7;
    pub const KEY_Greek_upsilonaccent: u32 = 0x07b8;
    pub const KEY_Greek_upsilondieresis: u32 = 0x07b9;
    pub const KEY_Greek_upsilonaccentdieresis: u32 = 0x07ba;
    pub const KEY_Greek_omegaaccent: u32 = 0x07bb;
    pub const KEY_Greek_ALPHA: u32 = 0x07c1;
    pub const KEY_Greek_BETA: u32 = 0x07c2;
    pub const KEY_Greek_GAMMA: u32 = 0x07c3;
    pub const KEY_Greek_DELTA: u32 = 0x07c4;
    pub const KEY_Greek_EPSILON: u32 = 0x07c5;
    pub const KEY_Greek_ZETA: u32 = 0x07c6;
    pub const KEY_Greek_ETA: u32 = 0x07c7;
    pub const KEY_Greek_THETA: u32 = 0x07c8;
    pub const KEY_Greek_IOTA: u32 = 0x07c9;
    pub const KEY_Greek_KAPPA: u32 = 0x07ca;
    pub const KEY_Greek_LAMDA: u32 = 0x07cb;
    pub const KEY_Greek_LAMBDA: u32 = 0x07cb;
    pub const KEY_Greek_MU: u32 = 0x07cc;
    pub const KEY_Greek_NU: u32 = 0x07cd;
    pub const KEY_Greek_XI: u32 = 0x07ce;
    pub const KEY_Greek_OMICRON: u32 = 0x07cf;
    pub const KEY_Greek_PI: u32 = 0x07d0;
    pub const KEY_Greek_RHO: u32 = 0x07d1;
    pub const KEY_Greek_SIGMA: u32 = 0x07d2;
    pub const KEY_Greek_TAU: u32 = 0x07d4;
    pub const KEY_Greek_UPSILON: u32 = 0x07d5;
    pub const KEY_Greek_PHI: u32 = 0x07d6;
    pub const KEY_Greek_CHI: u32 = 0x07d7;
    pub const KEY_Greek_PSI: u32 = 0x07d8;
    pub const KEY_Greek_OMEGA: u32 = 0x07d9;
    pub const KEY_Greek_alpha: u32 = 0x07e1;
    pub const KEY_Greek_beta: u32 = 0x07e2;
    pub const KEY_Greek_gamma: u32 = 0x07e3;
    pub const KEY_Greek_delta: u32 = 0x07e4;
    pub const KEY_Greek_epsilon: u32 = 0x07e5;
    pub const KEY_Greek_zeta: u32 = 0x07e6;
    pub const KEY_Greek_eta: u32 = 0x07e7;
    pub const KEY_Greek_theta: u32 = 0x07e8;
    pub const KEY_Greek_iota: u32 = 0x07e9;
    pub const KEY_Greek_kappa: u32 = 0x07ea;
    pub const KEY_Greek_lamda: u32 = 0x07eb;
    pub const KEY_Greek_lambda: u32 = 0x07eb;
    pub const KEY_Greek_mu: u32 = 0x07ec;
    pub const KEY_Greek_nu: u32 = 0x07ed;
    pub const KEY_Greek_xi: u32 = 0x07ee;
    pub const KEY_Greek_omicron: u32 = 0x07ef;
    pub const KEY_Greek_pi: u32 = 0x07f0;
    pub const KEY_Greek_rho: u32 = 0x07f1;
    pub const KEY_Greek_sigma: u32 = 0x07f2;
    pub const KEY_Greek_finalsmallsigma: u32 = 0x07f3;
    pub const KEY_Greek_tau: u32 = 0x07f4;
    pub const KEY_Greek_upsilon: u32 = 0x07f5;
    pub const KEY_Greek_phi: u32 = 0x07f6;
    pub const KEY_Greek_chi: u32 = 0x07f7;
    pub const KEY_Greek_psi: u32 = 0x07f8;
    pub const KEY_Greek_omega: u32 = 0x07f9;
    pub const KEY_Greek_switch: u32 = 0xff7e;
    pub const KEY_leftradical: u32 = 0x08a1;
    pub const KEY_topleftradical: u32 = 0x08a2;
    pub const KEY_horizconnector: u32 = 0x08a3;
    pub const KEY_topintegral: u32 = 0x08a4;
    pub const KEY_botintegral: u32 = 0x08a5;
    pub const KEY_vertconnector: u32 = 0x08a6;
    pub const KEY_topleftsqbracket: u32 = 0x08a7;
    pub const KEY_botleftsqbracket: u32 = 0x08a8;
    pub const KEY_toprightsqbracket: u32 = 0x08a9;
    pub const KEY_botrightsqbracket: u32 = 0x08aa;
    pub const KEY_topleftparens: u32 = 0x08ab;
    pub const KEY_botleftparens: u32 = 0x08ac;
    pub const KEY_toprightparens: u32 = 0x08ad;
    pub const KEY_botrightparens: u32 = 0x08ae;
    pub const KEY_leftmiddlecurlybrace: u32 = 0x08af;
    pub const KEY_rightmiddlecurlybrace: u32 = 0x08b0;
    pub const KEY_topleftsummation: u32 = 0x08b1;
    pub const KEY_botleftsummation: u32 = 0x08b2;
    pub const KEY_topvertsummationconnector: u32 = 0x08b3;
    pub const KEY_botvertsummationconnector: u32 = 0x08b4;
    pub const KEY_toprightsummation: u32 = 0x08b5;
    pub const KEY_botrightsummation: u32 = 0x08b6;
    pub const KEY_rightmiddlesummation: u32 = 0x08b7;
    pub const KEY_lessthanequal: u32 = 0x08bc;
    pub const KEY_notequal: u32 = 0x08bd;
    pub const KEY_greaterthanequal: u32 = 0x08be;
    pub const KEY_integral: u32 = 0x08bf;
    pub const KEY_therefore: u32 = 0x08c0;
    pub const KEY_variation: u32 = 0x08c1;
    pub const KEY_infinity: u32 = 0x08c2;
    pub const KEY_nabla: u32 = 0x08c5;
    pub const KEY_approximate: u32 = 0x08c8;
    pub const KEY_similarequal: u32 = 0x08c9;
    pub const KEY_ifonlyif: u32 = 0x08cd;
    pub const KEY_implies: u32 = 0x08ce;
    pub const KEY_identical: u32 = 0x08cf;
    pub const KEY_radical: u32 = 0x08d6;
    pub const KEY_includedin: u32 = 0x08da;
    pub const KEY_includes: u32 = 0x08db;
    pub const KEY_intersection: u32 = 0x08dc;
    pub const KEY_union: u32 = 0x08dd;
    pub const KEY_logicaland: u32 = 0x08de;
    pub const KEY_logicalor: u32 = 0x08df;
    pub const KEY_partialderivative: u32 = 0x08ef;
    pub const KEY_function: u32 = 0x08f6;
    pub const KEY_leftarrow: u32 = 0x08fb;
    pub const KEY_uparrow: u32 = 0x08fc;
    pub const KEY_rightarrow: u32 = 0x08fd;
    pub const KEY_downarrow: u32 = 0x08fe;
    pub const KEY_blank: u32 = 0x09df;
    pub const KEY_soliddiamond: u32 = 0x09e0;
    pub const KEY_checkerboard: u32 = 0x09e1;
    pub const KEY_ht: u32 = 0x09e2;
    pub const KEY_ff: u32 = 0x09e3;
    pub const KEY_cr: u32 = 0x09e4;
    pub const KEY_lf: u32 = 0x09e5;
    pub const KEY_nl: u32 = 0x09e8;
    pub const KEY_vt: u32 = 0x09e9;
    pub const KEY_lowrightcorner: u32 = 0x09ea;
    pub const KEY_uprightcorner: u32 = 0x09eb;
    pub const KEY_upleftcorner: u32 = 0x09ec;
    pub const KEY_lowleftcorner: u32 = 0x09ed;
    pub const KEY_crossinglines: u32 = 0x09ee;
    pub const KEY_horizlinescan1: u32 = 0x09ef;
    pub const KEY_horizlinescan3: u32 = 0x09f0;
    pub const KEY_horizlinescan5: u32 = 0x09f1;
    pub const KEY_horizlinescan7: u32 = 0x09f2;
    pub const KEY_horizlinescan9: u32 = 0x09f3;
    pub const KEY_leftt: u32 = 0x09f4;
    pub const KEY_rightt: u32 = 0x09f5;
    pub const KEY_bott: u32 = 0x09f6;
    pub const KEY_topt: u32 = 0x09f7;
    pub const KEY_vertbar: u32 = 0x09f8;
    pub const KEY_emspace: u32 = 0x0aa1;
    pub const KEY_enspace: u32 = 0x0aa2;
    pub const KEY_em3space: u32 = 0x0aa3;
    pub const KEY_em4space: u32 = 0x0aa4;
    pub const KEY_digitspace: u32 = 0x0aa5;
    pub const KEY_punctspace: u32 = 0x0aa6;
    pub const KEY_thinspace: u32 = 0x0aa7;
    pub const KEY_hairspace: u32 = 0x0aa8;
    pub const KEY_emdash: u32 = 0x0aa9;
    pub const KEY_endash: u32 = 0x0aaa;
    pub const KEY_signifblank: u32 = 0x0aac;
    pub const KEY_ellipsis: u32 = 0x0aae;
    pub const KEY_doubbaselinedot: u32 = 0x0aaf;
    pub const KEY_onethird: u32 = 0x0ab0;
    pub const KEY_twothirds: u32 = 0x0ab1;
    pub const KEY_onefifth: u32 = 0x0ab2;
    pub const KEY_twofifths: u32 = 0x0ab3;
    pub const KEY_threefifths: u32 = 0x0ab4;
    pub const KEY_fourfifths: u32 = 0x0ab5;
    pub const KEY_onesixth: u32 = 0x0ab6;
    pub const KEY_fivesixths: u32 = 0x0ab7;
    pub const KEY_careof: u32 = 0x0ab8;
    pub const KEY_figdash: u32 = 0x0abb;
    pub const KEY_leftanglebracket: u32 = 0x0abc;
    pub const KEY_decimalpoint: u32 = 0x0abd;
    pub const KEY_rightanglebracket: u32 = 0x0abe;
    pub const KEY_marker: u32 = 0x0abf;
    pub const KEY_oneeighth: u32 = 0x0ac3;
    pub const KEY_threeeighths: u32 = 0x0ac4;
    pub const KEY_fiveeighths: u32 = 0x0ac5;
    pub const KEY_seveneighths: u32 = 0x0ac6;
    pub const KEY_trademark: u32 = 0x0ac9;
    pub const KEY_signaturemark: u32 = 0x0aca;
    pub const KEY_trademarkincircle: u32 = 0x0acb;
    pub const KEY_leftopentriangle: u32 = 0x0acc;
    pub const KEY_rightopentriangle: u32 = 0x0acd;
    pub const KEY_emopencircle: u32 = 0x0ace;
    pub const KEY_emopenrectangle: u32 = 0x0acf;
    pub const KEY_leftsinglequotemark: u32 = 0x0ad0;
    pub const KEY_rightsinglequotemark: u32 = 0x0ad1;
    pub const KEY_leftdoublequotemark: u32 = 0x0ad2;
    pub const KEY_rightdoublequotemark: u32 = 0x0ad3;
    pub const KEY_prescription: u32 = 0x0ad4;
    pub const KEY_permille: u32 = 0x0ad5;
    pub const KEY_minutes: u32 = 0x0ad6;
    pub const KEY_seconds: u32 = 0x0ad7;
    pub const KEY_latincross: u32 = 0x0ad9;
    pub const KEY_hexagram: u32 = 0x0ada;
    pub const KEY_filledrectbullet: u32 = 0x0adb;
    pub const KEY_filledlefttribullet: u32 = 0x0adc;
    pub const KEY_filledrighttribullet: u32 = 0x0add;
    pub const KEY_emfilledcircle: u32 = 0x0ade;
    pub const KEY_emfilledrect: u32 = 0x0adf;
    pub const KEY_enopencircbullet: u32 = 0x0ae0;
    pub const KEY_enopensquarebullet: u32 = 0x0ae1;
    pub const KEY_openrectbullet: u32 = 0x0ae2;
    pub const KEY_opentribulletup: u32 = 0x0ae3;
    pub const KEY_opentribulletdown: u32 = 0x0ae4;
    pub const KEY_openstar: u32 = 0x0ae5;
    pub const KEY_enfilledcircbullet: u32 = 0x0ae6;
    pub const KEY_enfilledsqbullet: u32 = 0x0ae7;
    pub const KEY_filledtribulletup: u32 = 0x0ae8;
    pub const KEY_filledtribulletdown: u32 = 0x0ae9;
    pub const KEY_leftpointer: u32 = 0x0aea;
    pub const KEY_rightpointer: u32 = 0x0aeb;
    pub const KEY_club: u32 = 0x0aec;
    pub const KEY_diamond: u32 = 0x0aed;
    pub const KEY_heart: u32 = 0x0aee;
    pub const KEY_maltesecross: u32 = 0x0af0;
    pub const KEY_dagger: u32 = 0x0af1;
    pub const KEY_doubledagger: u32 = 0x0af2;
    pub const KEY_checkmark: u32 = 0x0af3;
    pub const KEY_ballotcross: u32 = 0x0af4;
    pub const KEY_musicalsharp: u32 = 0x0af5;
    pub const KEY_musicalflat: u32 = 0x0af6;
    pub const KEY_malesymbol: u32 = 0x0af7;
    pub const KEY_femalesymbol: u32 = 0x0af8;
    pub const KEY_telephone: u32 = 0x0af9;
    pub const KEY_telephonerecorder: u32 = 0x0afa;
    pub const KEY_phonographcopyright: u32 = 0x0afb;
    pub const KEY_caret: u32 = 0x0afc;
    pub const KEY_singlelowquotemark: u32 = 0x0afd;
    pub const KEY_doublelowquotemark: u32 = 0x0afe;
    pub const KEY_cursor: u32 = 0x0aff;
    pub const KEY_leftcaret: u32 = 0x0ba3;
    pub const KEY_rightcaret: u32 = 0x0ba6;
    pub const KEY_downcaret: u32 = 0x0ba8;
    pub const KEY_upcaret: u32 = 0x0ba9;
    pub const KEY_overbar: u32 = 0x0bc0;
    pub const KEY_downtack: u32 = 0x0bc2;
    pub const KEY_upshoe: u32 = 0x0bc3;
    pub const KEY_downstile: u32 = 0x0bc4;
    pub const KEY_underbar: u32 = 0x0bc6;
    pub const KEY_jot: u32 = 0x0bca;
    pub const KEY_quad: u32 = 0x0bcc;
    pub const KEY_uptack: u32 = 0x0bce;
    pub const KEY_circle: u32 = 0x0bcf;
    pub const KEY_upstile: u32 = 0x0bd3;
    pub const KEY_downshoe: u32 = 0x0bd6;
    pub const KEY_rightshoe: u32 = 0x0bd8;
    pub const KEY_leftshoe: u32 = 0x0bda;
    pub const KEY_lefttack: u32 = 0x0bdc;
    pub const KEY_righttack: u32 = 0x0bfc;
    pub const KEY_hebrew_doublelowline: u32 = 0x0cdf;
    pub const KEY_hebrew_aleph: u32 = 0x0ce0;
    pub const KEY_hebrew_bet: u32 = 0x0ce1;
    pub const KEY_hebrew_beth: u32 = 0x0ce1;
    pub const KEY_hebrew_gimel: u32 = 0x0ce2;
    pub const KEY_hebrew_gimmel: u32 = 0x0ce2;
    pub const KEY_hebrew_dalet: u32 = 0x0ce3;
    pub const KEY_hebrew_daleth: u32 = 0x0ce3;
    pub const KEY_hebrew_he: u32 = 0x0ce4;
    pub const KEY_hebrew_waw: u32 = 0x0ce5;
    pub const KEY_hebrew_zain: u32 = 0x0ce6;
    pub const KEY_hebrew_zayin: u32 = 0x0ce6;
    pub const KEY_hebrew_chet: u32 = 0x0ce7;
    pub const KEY_hebrew_het: u32 = 0x0ce7;
    pub const KEY_hebrew_tet: u32 = 0x0ce8;
    pub const KEY_hebrew_teth: u32 = 0x0ce8;
    pub const KEY_hebrew_yod: u32 = 0x0ce9;
    pub const KEY_hebrew_finalkaph: u32 = 0x0cea;
    pub const KEY_hebrew_kaph: u32 = 0x0ceb;
    pub const KEY_hebrew_lamed: u32 = 0x0cec;
    pub const KEY_hebrew_finalmem: u32 = 0x0ced;
    pub const KEY_hebrew_mem: u32 = 0x0cee;
    pub const KEY_hebrew_finalnun: u32 = 0x0cef;
    pub const KEY_hebrew_nun: u32 = 0x0cf0;
    pub const KEY_hebrew_samech: u32 = 0x0cf1;
    pub const KEY_hebrew_samekh: u32 = 0x0cf1;
    pub const KEY_hebrew_ayin: u32 = 0x0cf2;
    pub const KEY_hebrew_finalpe: u32 = 0x0cf3;
    pub const KEY_hebrew_pe: u32 = 0x0cf4;
    pub const KEY_hebrew_finalzade: u32 = 0x0cf5;
    pub const KEY_hebrew_finalzadi: u32 = 0x0cf5;
    pub const KEY_hebrew_zade: u32 = 0x0cf6;
    pub const KEY_hebrew_zadi: u32 = 0x0cf6;
    pub const KEY_hebrew_qoph: u32 = 0x0cf7;
    pub const KEY_hebrew_kuf: u32 = 0x0cf7;
    pub const KEY_hebrew_resh: u32 = 0x0cf8;
    pub const KEY_hebrew_shin: u32 = 0x0cf9;
    pub const KEY_hebrew_taw: u32 = 0x0cfa;
    pub const KEY_hebrew_taf: u32 = 0x0cfa;
    pub const KEY_Hebrew_switch: u32 = 0xff7e;
    pub const KEY_Thai_kokai: u32 = 0x0da1;
    pub const KEY_Thai_khokhai: u32 = 0x0da2;
    pub const KEY_Thai_khokhuat: u32 = 0x0da3;
    pub const KEY_Thai_khokhwai: u32 = 0x0da4;
    pub const KEY_Thai_khokhon: u32 = 0x0da5;
    pub const KEY_Thai_khorakhang: u32 = 0x0da6;
    pub const KEY_Thai_ngongu: u32 = 0x0da7;
    pub const KEY_Thai_chochan: u32 = 0x0da8;
    pub const KEY_Thai_choching: u32 = 0x0da9;
    pub const KEY_Thai_chochang: u32 = 0x0daa;
    pub const KEY_Thai_soso: u32 = 0x0dab;
    pub const KEY_Thai_chochoe: u32 = 0x0dac;
    pub const KEY_Thai_yoying: u32 = 0x0dad;
    pub const KEY_Thai_dochada: u32 = 0x0dae;
    pub const KEY_Thai_topatak: u32 = 0x0daf;
    pub const KEY_Thai_thothan: u32 = 0x0db0;
    pub const KEY_Thai_thonangmontho: u32 = 0x0db1;
    pub const KEY_Thai_thophuthao: u32 = 0x0db2;
    pub const KEY_Thai_nonen: u32 = 0x0db3;
    pub const KEY_Thai_dodek: u32 = 0x0db4;
    pub const KEY_Thai_totao: u32 = 0x0db5;
    pub const KEY_Thai_thothung: u32 = 0x0db6;
    pub const KEY_Thai_thothahan: u32 = 0x0db7;
    pub const KEY_Thai_thothong: u32 = 0x0db8;
    pub const KEY_Thai_nonu: u32 = 0x0db9;
    pub const KEY_Thai_bobaimai: u32 = 0x0dba;
    pub const KEY_Thai_popla: u32 = 0x0dbb;
    pub const KEY_Thai_phophung: u32 = 0x0dbc;
    pub const KEY_Thai_fofa: u32 = 0x0dbd;
    pub const KEY_Thai_phophan: u32 = 0x0dbe;
    pub const KEY_Thai_fofan: u32 = 0x0dbf;
    pub const KEY_Thai_phosamphao: u32 = 0x0dc0;
    pub const KEY_Thai_moma: u32 = 0x0dc1;
    pub const KEY_Thai_yoyak: u32 = 0x0dc2;
    pub const KEY_Thai_rorua: u32 = 0x0dc3;
    pub const KEY_Thai_ru: u32 = 0x0dc4;
    pub const KEY_Thai_loling: u32 = 0x0dc5;
    pub const KEY_Thai_lu: u32 = 0x0dc6;
    pub const KEY_Thai_wowaen: u32 = 0x0dc7;
    pub const KEY_Thai_sosala: u32 = 0x0dc8;
    pub const KEY_Thai_sorusi: u32 = 0x0dc9;
    pub const KEY_Thai_sosua: u32 = 0x0dca;
    pub const KEY_Thai_hohip: u32 = 0x0dcb;
    pub const KEY_Thai_lochula: u32 = 0x0dcc;
    pub const KEY_Thai_oang: u32 = 0x0dcd;
    pub const KEY_Thai_honokhuk: u32 = 0x0dce;
    pub const KEY_Thai_paiyannoi: u32 = 0x0dcf;
    pub const KEY_Thai_saraa: u32 = 0x0dd0;
    pub const KEY_Thai_maihanakat: u32 = 0x0dd1;
    pub const KEY_Thai_saraaa: u32 = 0x0dd2;
    pub const KEY_Thai_saraam: u32 = 0x0dd3;
    pub const KEY_Thai_sarai: u32 = 0x0dd4;
    pub const KEY_Thai_saraii: u32 = 0x0dd5;
    pub const KEY_Thai_saraue: u32 = 0x0dd6;
    pub const KEY_Thai_sarauee: u32 = 0x0dd7;
    pub const KEY_Thai_sarau: u32 = 0x0dd8;
    pub const KEY_Thai_sarauu: u32 = 0x0dd9;
    pub const KEY_Thai_phinthu: u32 = 0x0dda;
    pub const KEY_Thai_maihanakat_maitho: u32 = 0x0dde;
    pub const KEY_Thai_baht: u32 = 0x0ddf;
    pub const KEY_Thai_sarae: u32 = 0x0de0;
    pub const KEY_Thai_saraae: u32 = 0x0de1;
    pub const KEY_Thai_sarao: u32 = 0x0de2;
    pub const KEY_Thai_saraaimaimuan: u32 = 0x0de3;
    pub const KEY_Thai_saraaimaimalai: u32 = 0x0de4;
    pub const KEY_Thai_lakkhangyao: u32 = 0x0de5;
    pub const KEY_Thai_maiyamok: u32 = 0x0de6;
    pub const KEY_Thai_maitaikhu: u32 = 0x0de7;
    pub const KEY_Thai_maiek: u32 = 0x0de8;
    pub const KEY_Thai_maitho: u32 = 0x0de9;
    pub const KEY_Thai_maitri: u32 = 0x0dea;
    pub const KEY_Thai_maichattawa: u32 = 0x0deb;
    pub const KEY_Thai_thanthakhat: u32 = 0x0dec;
    pub const KEY_Thai_nikhahit: u32 = 0x0ded;
    pub const KEY_Thai_leksun: u32 = 0x0df0;
    pub const KEY_Thai_leknung: u32 = 0x0df1;
    pub const KEY_Thai_leksong: u32 = 0x0df2;
    pub const KEY_Thai_leksam: u32 = 0x0df3;
    pub const KEY_Thai_leksi: u32 = 0x0df4;
    pub const KEY_Thai_lekha: u32 = 0x0df5;
    pub const KEY_Thai_lekhok: u32 = 0x0df6;
    pub const KEY_Thai_lekchet: u32 = 0x0df7;
    pub const KEY_Thai_lekpaet: u32 = 0x0df8;
    pub const KEY_Thai_lekkao: u32 = 0x0df9;
    pub const KEY_Hangul: u32 = 0xff31;
    pub const KEY_Hangul_Start: u32 = 0xff32;
    pub const KEY_Hangul_End: u32 = 0xff33;
    pub const KEY_Hangul_Hanja: u32 = 0xff34;
    pub const KEY_Hangul_Jamo: u32 = 0xff35;
    pub const KEY_Hangul_Romaja: u32 = 0xff36;
    pub const KEY_Hangul_Codeinput: u32 = 0xff37;
    pub const KEY_Hangul_Jeonja: u32 = 0xff38;
    pub const KEY_Hangul_Banja: u32 = 0xff39;
    pub const KEY_Hangul_PreHanja: u32 = 0xff3a;
    pub const KEY_Hangul_PostHanja: u32 = 0xff3b;
    pub const KEY_Hangul_SingleCandidate: u32 = 0xff3c;
    pub const KEY_Hangul_MultipleCandidate: u32 = 0xff3d;
    pub const KEY_Hangul_PreviousCandidate: u32 = 0xff3e;
    pub const KEY_Hangul_Special: u32 = 0xff3f;
    pub const KEY_Hangul_switch: u32 = 0xff7e;
    pub const KEY_Hangul_Kiyeog: u32 = 0x0ea1;
    pub const KEY_Hangul_SsangKiyeog: u32 = 0x0ea2;
    pub const KEY_Hangul_KiyeogSios: u32 = 0x0ea3;
    pub const KEY_Hangul_Nieun: u32 = 0x0ea4;
    pub const KEY_Hangul_NieunJieuj: u32 = 0x0ea5;
    pub const KEY_Hangul_NieunHieuh: u32 = 0x0ea6;
    pub const KEY_Hangul_Dikeud: u32 = 0x0ea7;
    pub const KEY_Hangul_SsangDikeud: u32 = 0x0ea8;
    pub const KEY_Hangul_Rieul: u32 = 0x0ea9;
    pub const KEY_Hangul_RieulKiyeog: u32 = 0x0eaa;
    pub const KEY_Hangul_RieulMieum: u32 = 0x0eab;
    pub const KEY_Hangul_RieulPieub: u32 = 0x0eac;
    pub const KEY_Hangul_RieulSios: u32 = 0x0ead;
    pub const KEY_Hangul_RieulTieut: u32 = 0x0eae;
    pub const KEY_Hangul_RieulPhieuf: u32 = 0x0eaf;
    pub const KEY_Hangul_RieulHieuh: u32 = 0x0eb0;
    pub const KEY_Hangul_Mieum: u32 = 0x0eb1;
    pub const KEY_Hangul_Pieub: u32 = 0x0eb2;
    pub const KEY_Hangul_SsangPieub: u32 = 0x0eb3;
    pub const KEY_Hangul_PieubSios: u32 = 0x0eb4;
    pub const KEY_Hangul_Sios: u32 = 0x0eb5;
    pub const KEY_Hangul_SsangSios: u32 = 0x0eb6;
    pub const KEY_Hangul_Ieung: u32 = 0x0eb7;
    pub const KEY_Hangul_Jieuj: u32 = 0x0eb8;
    pub const KEY_Hangul_SsangJieuj: u32 = 0x0eb9;
    pub const KEY_Hangul_Cieuc: u32 = 0x0eba;
    pub const KEY_Hangul_Khieuq: u32 = 0x0ebb;
    pub const KEY_Hangul_Tieut: u32 = 0x0ebc;
    pub const KEY_Hangul_Phieuf: u32 = 0x0ebd;
    pub const KEY_Hangul_Hieuh: u32 = 0x0ebe;
    pub const KEY_Hangul_A: u32 = 0x0ebf;
    pub const KEY_Hangul_AE: u32 = 0x0ec0;
    pub const KEY_Hangul_YA: u32 = 0x0ec1;
    pub const KEY_Hangul_YAE: u32 = 0x0ec2;
    pub const KEY_Hangul_EO: u32 = 0x0ec3;
    pub const KEY_Hangul_E: u32 = 0x0ec4;
    pub const KEY_Hangul_YEO: u32 = 0x0ec5;
    pub const KEY_Hangul_YE: u32 = 0x0ec6;
    pub const KEY_Hangul_O: u32 = 0x0ec7;
    pub const KEY_Hangul_WA: u32 = 0x0ec8;
    pub const KEY_Hangul_WAE: u32 = 0x0ec9;
    pub const KEY_Hangul_OE: u32 = 0x0eca;
    pub const KEY_Hangul_YO: u32 = 0x0ecb;
    pub const KEY_Hangul_U: u32 = 0x0ecc;
    pub const KEY_Hangul_WEO: u32 = 0x0ecd;
    pub const KEY_Hangul_WE: u32 = 0x0ece;
    pub const KEY_Hangul_WI: u32 = 0x0ecf;
    pub const KEY_Hangul_YU: u32 = 0x0ed0;
    pub const KEY_Hangul_EU: u32 = 0x0ed1;
    pub const KEY_Hangul_YI: u32 = 0x0ed2;
    pub const KEY_Hangul_I: u32 = 0x0ed3;
    pub const KEY_Hangul_J_Kiyeog: u32 = 0x0ed4;
    pub const KEY_Hangul_J_SsangKiyeog: u32 = 0x0ed5;
    pub const KEY_Hangul_J_KiyeogSios: u32 = 0x0ed6;
    pub const KEY_Hangul_J_Nieun: u32 = 0x0ed7;
    pub const KEY_Hangul_J_NieunJieuj: u32 = 0x0ed8;
    pub const KEY_Hangul_J_NieunHieuh: u32 = 0x0ed9;
    pub const KEY_Hangul_J_Dikeud: u32 = 0x0eda;
    pub const KEY_Hangul_J_Rieul: u32 = 0x0edb;
    pub const KEY_Hangul_J_RieulKiyeog: u32 = 0x0edc;
    pub const KEY_Hangul_J_RieulMieum: u32 = 0x0edd;
    pub const KEY_Hangul_J_RieulPieub: u32 = 0x0ede;
    pub const KEY_Hangul_J_RieulSios: u32 = 0x0edf;
    pub const KEY_Hangul_J_RieulTieut: u32 = 0x0ee0;
    pub const KEY_Hangul_J_RieulPhieuf: u32 = 0x0ee1;
    pub const KEY_Hangul_J_RieulHieuh: u32 = 0x0ee2;
    pub const KEY_Hangul_J_Mieum: u32 = 0x0ee3;
    pub const KEY_Hangul_J_Pieub: u32 = 0x0ee4;
    pub const KEY_Hangul_J_PieubSios: u32 = 0x0ee5;
    pub const KEY_Hangul_J_Sios: u32 = 0x0ee6;
    pub const KEY_Hangul_J_SsangSios: u32 = 0x0ee7;
    pub const KEY_Hangul_J_Ieung: u32 = 0x0ee8;
    pub const KEY_Hangul_J_Jieuj: u32 = 0x0ee9;
    pub const KEY_Hangul_J_Cieuc: u32 = 0x0eea;
    pub const KEY_Hangul_J_Khieuq: u32 = 0x0eeb;
    pub const KEY_Hangul_J_Tieut: u32 = 0x0eec;
    pub const KEY_Hangul_J_Phieuf: u32 = 0x0eed;
    pub const KEY_Hangul_J_Hieuh: u32 = 0x0eee;
    pub const KEY_Hangul_RieulYeorinHieuh: u32 = 0x0eef;
    pub const KEY_Hangul_SunkyeongeumMieum: u32 = 0x0ef0;
    pub const KEY_Hangul_SunkyeongeumPieub: u32 = 0x0ef1;
    pub const KEY_Hangul_PanSios: u32 = 0x0ef2;
    pub const KEY_Hangul_KkogjiDalrinIeung: u32 = 0x0ef3;
    pub const KEY_Hangul_SunkyeongeumPhieuf: u32 = 0x0ef4;
    pub const KEY_Hangul_YeorinHieuh: u32 = 0x0ef5;
    pub const KEY_Hangul_AraeA: u32 = 0x0ef6;
    pub const KEY_Hangul_AraeAE: u32 = 0x0ef7;
    pub const KEY_Hangul_J_PanSios: u32 = 0x0ef8;
    pub const KEY_Hangul_J_KkogjiDalrinIeung: u32 = 0x0ef9;
    pub const KEY_Hangul_J_YeorinHieuh: u32 = 0x0efa;
    pub const KEY_Korean_Won: u32 = 0x0eff;
    pub const KEY_Armenian_ligature_ew: u32 = 0x1000587;
    pub const KEY_Armenian_full_stop: u32 = 0x1000589;
    pub const KEY_Armenian_verjaket: u32 = 0x1000589;
    pub const KEY_Armenian_separation_mark: u32 = 0x100055d;
    pub const KEY_Armenian_but: u32 = 0x100055d;
    pub const KEY_Armenian_hyphen: u32 = 0x100058a;
    pub const KEY_Armenian_yentamna: u32 = 0x100058a;
    pub const KEY_Armenian_exclam: u32 = 0x100055c;
    pub const KEY_Armenian_amanak: u32 = 0x100055c;
    pub const KEY_Armenian_accent: u32 = 0x100055b;
    pub const KEY_Armenian_shesht: u32 = 0x100055b;
    pub const KEY_Armenian_question: u32 = 0x100055e;
    pub const KEY_Armenian_paruyk: u32 = 0x100055e;
    pub const KEY_Armenian_AYB: u32 = 0x1000531;
    pub const KEY_Armenian_ayb: u32 = 0x1000561;
    pub const KEY_Armenian_BEN: u32 = 0x1000532;
    pub const KEY_Armenian_ben: u32 = 0x1000562;
    pub const KEY_Armenian_GIM: u32 = 0x1000533;
    pub const KEY_Armenian_gim: u32 = 0x1000563;
    pub const KEY_Armenian_DA: u32 = 0x1000534;
    pub const KEY_Armenian_da: u32 = 0x1000564;
    pub const KEY_Armenian_YECH: u32 = 0x1000535;
    pub const KEY_Armenian_yech: u32 = 0x1000565;
    pub const KEY_Armenian_ZA: u32 = 0x1000536;
    pub const KEY_Armenian_za: u32 = 0x1000566;
    pub const KEY_Armenian_E: u32 = 0x1000537;
    pub const KEY_Armenian_e: u32 = 0x1000567;
    pub const KEY_Armenian_AT: u32 = 0x1000538;
    pub const KEY_Armenian_at: u32 = 0x1000568;
    pub const KEY_Armenian_TO: u32 = 0x1000539;
    pub const KEY_Armenian_to: u32 = 0x1000569;
    pub const KEY_Armenian_ZHE: u32 = 0x100053a;
    pub const KEY_Armenian_zhe: u32 = 0x100056a;
    pub const KEY_Armenian_INI: u32 = 0x100053b;
    pub const KEY_Armenian_ini: u32 = 0x100056b;
    pub const KEY_Armenian_LYUN: u32 = 0x100053c;
    pub const KEY_Armenian_lyun: u32 = 0x100056c;
    pub const KEY_Armenian_KHE: u32 = 0x100053d;
    pub const KEY_Armenian_khe: u32 = 0x100056d;
    pub const KEY_Armenian_TSA: u32 = 0x100053e;
    pub const KEY_Armenian_tsa: u32 = 0x100056e;
    pub const KEY_Armenian_KEN: u32 = 0x100053f;
    pub const KEY_Armenian_ken: u32 = 0x100056f;
    pub const KEY_Armenian_HO: u32 = 0x1000540;
    pub const KEY_Armenian_ho: u32 = 0x1000570;
    pub const KEY_Armenian_DZA: u32 = 0x1000541;
    pub const KEY_Armenian_dza: u32 = 0x1000571;
    pub const KEY_Armenian_GHAT: u32 = 0x1000542;
    pub const KEY_Armenian_ghat: u32 = 0x1000572;
    pub const KEY_Armenian_TCHE: u32 = 0x1000543;
    pub const KEY_Armenian_tche: u32 = 0x1000573;
    pub const KEY_Armenian_MEN: u32 = 0x1000544;
    pub const KEY_Armenian_men: u32 = 0x1000574;
    pub const KEY_Armenian_HI: u32 = 0x1000545;
    pub const KEY_Armenian_hi: u32 = 0x1000575;
    pub const KEY_Armenian_NU: u32 = 0x1000546;
    pub const KEY_Armenian_nu: u32 = 0x1000576;
    pub const KEY_Armenian_SHA: u32 = 0x1000547;
    pub const KEY_Armenian_sha: u32 = 0x1000577;
    pub const KEY_Armenian_VO: u32 = 0x1000548;
    pub const KEY_Armenian_vo: u32 = 0x1000578;
    pub const KEY_Armenian_CHA: u32 = 0x1000549;
    pub const KEY_Armenian_cha: u32 = 0x1000579;
    pub const KEY_Armenian_PE: u32 = 0x100054a;
    pub const KEY_Armenian_pe: u32 = 0x100057a;
    pub const KEY_Armenian_JE: u32 = 0x100054b;
    pub const KEY_Armenian_je: u32 = 0x100057b;
    pub const KEY_Armenian_RA: u32 = 0x100054c;
    pub const KEY_Armenian_ra: u32 = 0x100057c;
    pub const KEY_Armenian_SE: u32 = 0x100054d;
    pub const KEY_Armenian_se: u32 = 0x100057d;
    pub const KEY_Armenian_VEV: u32 = 0x100054e;
    pub const KEY_Armenian_vev: u32 = 0x100057e;
    pub const KEY_Armenian_TYUN: u32 = 0x100054f;
    pub const KEY_Armenian_tyun: u32 = 0x100057f;
    pub const KEY_Armenian_RE: u32 = 0x1000550;
    pub const KEY_Armenian_re: u32 = 0x1000580;
    pub const KEY_Armenian_TSO: u32 = 0x1000551;
    pub const KEY_Armenian_tso: u32 = 0x1000581;
    pub const KEY_Armenian_VYUN: u32 = 0x1000552;
    pub const KEY_Armenian_vyun: u32 = 0x1000582;
    pub const KEY_Armenian_PYUR: u32 = 0x1000553;
    pub const KEY_Armenian_pyur: u32 = 0x1000583;
    pub const KEY_Armenian_KE: u32 = 0x1000554;
    pub const KEY_Armenian_ke: u32 = 0x1000584;
    pub const KEY_Armenian_O: u32 = 0x1000555;
    pub const KEY_Armenian_o: u32 = 0x1000585;
    pub const KEY_Armenian_FE: u32 = 0x1000556;
    pub const KEY_Armenian_fe: u32 = 0x1000586;
    pub const KEY_Armenian_apostrophe: u32 = 0x100055a;
    pub const KEY_Georgian_an: u32 = 0x10010d0;
    pub const KEY_Georgian_ban: u32 = 0x10010d1;
    pub const KEY_Georgian_gan: u32 = 0x10010d2;
    pub const KEY_Georgian_don: u32 = 0x10010d3;
    pub const KEY_Georgian_en: u32 = 0x10010d4;
    pub const KEY_Georgian_vin: u32 = 0x10010d5;
    pub const KEY_Georgian_zen: u32 = 0x10010d6;
    pub const KEY_Georgian_tan: u32 = 0x10010d7;
    pub const KEY_Georgian_in: u32 = 0x10010d8;
    pub const KEY_Georgian_kan: u32 = 0x10010d9;
    pub const KEY_Georgian_las: u32 = 0x10010da;
    pub const KEY_Georgian_man: u32 = 0x10010db;
    pub const KEY_Georgian_nar: u32 = 0x10010dc;
    pub const KEY_Georgian_on: u32 = 0x10010dd;
    pub const KEY_Georgian_par: u32 = 0x10010de;
    pub const KEY_Georgian_zhar: u32 = 0x10010df;
    pub const KEY_Georgian_rae: u32 = 0x10010e0;
    pub const KEY_Georgian_san: u32 = 0x10010e1;
    pub const KEY_Georgian_tar: u32 = 0x10010e2;
    pub const KEY_Georgian_un: u32 = 0x10010e3;
    pub const KEY_Georgian_phar: u32 = 0x10010e4;
    pub const KEY_Georgian_khar: u32 = 0x10010e5;
    pub const KEY_Georgian_ghan: u32 = 0x10010e6;
    pub const KEY_Georgian_qar: u32 = 0x10010e7;
    pub const KEY_Georgian_shin: u32 = 0x10010e8;
    pub const KEY_Georgian_chin: u32 = 0x10010e9;
    pub const KEY_Georgian_can: u32 = 0x10010ea;
    pub const KEY_Georgian_jil: u32 = 0x10010eb;
    pub const KEY_Georgian_cil: u32 = 0x10010ec;
    pub const KEY_Georgian_char: u32 = 0x10010ed;
    pub const KEY_Georgian_xan: u32 = 0x10010ee;
    pub const KEY_Georgian_jhan: u32 = 0x10010ef;
    pub const KEY_Georgian_hae: u32 = 0x10010f0;
    pub const KEY_Georgian_he: u32 = 0x10010f1;
    pub const KEY_Georgian_hie: u32 = 0x10010f2;
    pub const KEY_Georgian_we: u32 = 0x10010f3;
    pub const KEY_Georgian_har: u32 = 0x10010f4;
    pub const KEY_Georgian_hoe: u32 = 0x10010f5;
    pub const KEY_Georgian_fi: u32 = 0x10010f6;
    pub const KEY_Xabovedot: u32 = 0x1001e8a;
    pub const KEY_Ibreve: u32 = 0x100012c;
    pub const KEY_Zstroke: u32 = 0x10001b5;
    pub const KEY_Gcaron: u32 = 0x10001e6;
    pub const KEY_Ocaron: u32 = 0x10001d1;
    pub const KEY_Obarred: u32 = 0x100019f;
    pub const KEY_xabovedot: u32 = 0x1001e8b;
    pub const KEY_ibreve: u32 = 0x100012d;
    pub const KEY_zstroke: u32 = 0x10001b6;
    pub const KEY_gcaron: u32 = 0x10001e7;
    pub const KEY_ocaron: u32 = 0x10001d2;
    pub const KEY_obarred: u32 = 0x1000275;
    pub const KEY_SCHWA: u32 = 0x100018f;
    pub const KEY_schwa: u32 = 0x1000259;
    pub const KEY_EZH: u32 = 0x10001b7;
    pub const KEY_ezh: u32 = 0x1000292;
    pub const KEY_Lbelowdot: u32 = 0x1001e36;
    pub const KEY_lbelowdot: u32 = 0x1001e37;
    pub const KEY_Abelowdot: u32 = 0x1001ea0;
    pub const KEY_abelowdot: u32 = 0x1001ea1;
    pub const KEY_Ahook: u32 = 0x1001ea2;
    pub const KEY_ahook: u32 = 0x1001ea3;
    pub const KEY_Acircumflexacute: u32 = 0x1001ea4;
    pub const KEY_acircumflexacute: u32 = 0x1001ea5;
    pub const KEY_Acircumflexgrave: u32 = 0x1001ea6;
    pub const KEY_acircumflexgrave: u32 = 0x1001ea7;
    pub const KEY_Acircumflexhook: u32 = 0x1001ea8;
    pub const KEY_acircumflexhook: u32 = 0x1001ea9;
    pub const KEY_Acircumflextilde: u32 = 0x1001eaa;
    pub const KEY_acircumflextilde: u32 = 0x1001eab;
    pub const KEY_Acircumflexbelowdot: u32 = 0x1001eac;
    pub const KEY_acircumflexbelowdot: u32 = 0x1001ead;
    pub const KEY_Abreveacute: u32 = 0x1001eae;
    pub const KEY_abreveacute: u32 = 0x1001eaf;
    pub const KEY_Abrevegrave: u32 = 0x1001eb0;
    pub const KEY_abrevegrave: u32 = 0x1001eb1;
    pub const KEY_Abrevehook: u32 = 0x1001eb2;
    pub const KEY_abrevehook: u32 = 0x1001eb3;
    pub const KEY_Abrevetilde: u32 = 0x1001eb4;
    pub const KEY_abrevetilde: u32 = 0x1001eb5;
    pub const KEY_Abrevebelowdot: u32 = 0x1001eb6;
    pub const KEY_abrevebelowdot: u32 = 0x1001eb7;
    pub const KEY_Ebelowdot: u32 = 0x1001eb8;
    pub const KEY_ebelowdot: u32 = 0x1001eb9;
    pub const KEY_Ehook: u32 = 0x1001eba;
    pub const KEY_ehook: u32 = 0x1001ebb;
    pub const KEY_Etilde: u32 = 0x1001ebc;
    pub const KEY_etilde: u32 = 0x1001ebd;
    pub const KEY_Ecircumflexacute: u32 = 0x1001ebe;
    pub const KEY_ecircumflexacute: u32 = 0x1001ebf;
    pub const KEY_Ecircumflexgrave: u32 = 0x1001ec0;
    pub const KEY_ecircumflexgrave: u32 = 0x1001ec1;
    pub const KEY_Ecircumflexhook: u32 = 0x1001ec2;
    pub const KEY_ecircumflexhook: u32 = 0x1001ec3;
    pub const KEY_Ecircumflextilde: u32 = 0x1001ec4;
    pub const KEY_ecircumflextilde: u32 = 0x1001ec5;
    pub const KEY_Ecircumflexbelowdot: u32 = 0x1001ec6;
    pub const KEY_ecircumflexbelowdot: u32 = 0x1001ec7;
    pub const KEY_Ihook: u32 = 0x1001ec8;
    pub const KEY_ihook: u32 = 0x1001ec9;
    pub const KEY_Ibelowdot: u32 = 0x1001eca;
    pub const KEY_ibelowdot: u32 = 0x1001ecb;
    pub const KEY_Obelowdot: u32 = 0x1001ecc;
    pub const KEY_obelowdot: u32 = 0x1001ecd;
    pub const KEY_Ohook: u32 = 0x1001ece;
    pub const KEY_ohook: u32 = 0x1001ecf;
    pub const KEY_Ocircumflexacute: u32 = 0x1001ed0;
    pub const KEY_ocircumflexacute: u32 = 0x1001ed1;
    pub const KEY_Ocircumflexgrave: u32 = 0x1001ed2;
    pub const KEY_ocircumflexgrave: u32 = 0x1001ed3;
    pub const KEY_Ocircumflexhook: u32 = 0x1001ed4;
    pub const KEY_ocircumflexhook: u32 = 0x1001ed5;
    pub const KEY_Ocircumflextilde: u32 = 0x1001ed6;
    pub const KEY_ocircumflextilde: u32 = 0x1001ed7;
    pub const KEY_Ocircumflexbelowdot: u32 = 0x1001ed8;
    pub const KEY_ocircumflexbelowdot: u32 = 0x1001ed9;
    pub const KEY_Ohornacute: u32 = 0x1001eda;
    pub const KEY_ohornacute: u32 = 0x1001edb;
    pub const KEY_Ohorngrave: u32 = 0x1001edc;
    pub const KEY_ohorngrave: u32 = 0x1001edd;
    pub const KEY_Ohornhook: u32 = 0x1001ede;
    pub const KEY_ohornhook: u32 = 0x1001edf;
    pub const KEY_Ohorntilde: u32 = 0x1001ee0;
    pub const KEY_ohorntilde: u32 = 0x1001ee1;
    pub const KEY_Ohornbelowdot: u32 = 0x1001ee2;
    pub const KEY_ohornbelowdot: u32 = 0x1001ee3;
    pub const KEY_Ubelowdot: u32 = 0x1001ee4;
    pub const KEY_ubelowdot: u32 = 0x1001ee5;
    pub const KEY_Uhook: u32 = 0x1001ee6;
    pub const KEY_uhook: u32 = 0x1001ee7;
    pub const KEY_Uhornacute: u32 = 0x1001ee8;
    pub const KEY_uhornacute: u32 = 0x1001ee9;
    pub const KEY_Uhorngrave: u32 = 0x1001eea;
    pub const KEY_uhorngrave: u32 = 0x1001eeb;
    pub const KEY_Uhornhook: u32 = 0x1001eec;
    pub const KEY_uhornhook: u32 = 0x1001eed;
    pub const KEY_Uhorntilde: u32 = 0x1001eee;
    pub const KEY_uhorntilde: u32 = 0x1001eef;
    pub const KEY_Uhornbelowdot: u32 = 0x1001ef0;
    pub const KEY_uhornbelowdot: u32 = 0x1001ef1;
    pub const KEY_Ybelowdot: u32 = 0x1001ef4;
    pub const KEY_ybelowdot: u32 = 0x1001ef5;
    pub const KEY_Yhook: u32 = 0x1001ef6;
    pub const KEY_yhook: u32 = 0x1001ef7;
    pub const KEY_Ytilde: u32 = 0x1001ef8;
    pub const KEY_ytilde: u32 = 0x1001ef9;
    pub const KEY_Ohorn: u32 = 0x10001a0;
    pub const KEY_ohorn: u32 = 0x10001a1;
    pub const KEY_Uhorn: u32 = 0x10001af;
    pub const KEY_uhorn: u32 = 0x10001b0;
    pub const KEY_combining_tilde: u32 = 0x1000303;
    pub const KEY_combining_grave: u32 = 0x1000300;
    pub const KEY_combining_acute: u32 = 0x1000301;
    pub const KEY_combining_hook: u32 = 0x1000309;
    pub const KEY_combining_belowdot: u32 = 0x1000323;
    pub const KEY_EcuSign: u32 = 0x10020a0;
    pub const KEY_ColonSign: u32 = 0x10020a1;
    pub const KEY_CruzeiroSign: u32 = 0x10020a2;
    pub const KEY_FFrancSign: u32 = 0x10020a3;
    pub const KEY_LiraSign: u32 = 0x10020a4;
    pub const KEY_MillSign: u32 = 0x10020a5;
    pub const KEY_NairaSign: u32 = 0x10020a6;
    pub const KEY_PesetaSign: u32 = 0x10020a7;
    pub const KEY_RupeeSign: u32 = 0x10020a8;
    pub const KEY_WonSign: u32 = 0x10020a9;
    pub const KEY_NewSheqelSign: u32 = 0x10020aa;
    pub const KEY_DongSign: u32 = 0x10020ab;
    pub const KEY_EuroSign: u32 = 0x20ac;
    pub const KEY_zerosuperior: u32 = 0x1002070;
    pub const KEY_foursuperior: u32 = 0x1002074;
    pub const KEY_fivesuperior: u32 = 0x1002075;
    pub const KEY_sixsuperior: u32 = 0x1002076;
    pub const KEY_sevensuperior: u32 = 0x1002077;
    pub const KEY_eightsuperior: u32 = 0x1002078;
    pub const KEY_ninesuperior: u32 = 0x1002079;
    pub const KEY_zerosubscript: u32 = 0x1002080;
    pub const KEY_onesubscript: u32 = 0x1002081;
    pub const KEY_twosubscript: u32 = 0x1002082;
    pub const KEY_threesubscript: u32 = 0x1002083;
    pub const KEY_foursubscript: u32 = 0x1002084;
    pub const KEY_fivesubscript: u32 = 0x1002085;
    pub const KEY_sixsubscript: u32 = 0x1002086;
    pub const KEY_sevensubscript: u32 = 0x1002087;
    pub const KEY_eightsubscript: u32 = 0x1002088;
    pub const KEY_ninesubscript: u32 = 0x1002089;
    pub const KEY_partdifferential: u32 = 0x1002202;
    pub const KEY_emptyset: u32 = 0x1002205;
    pub const KEY_elementof: u32 = 0x1002208;
    pub const KEY_notelementof: u32 = 0x1002209;
    pub const KEY_containsas: u32 = 0x100220b;
    pub const KEY_squareroot: u32 = 0x100221a;
    pub const KEY_cuberoot: u32 = 0x100221b;
    pub const KEY_fourthroot: u32 = 0x100221c;
    pub const KEY_dintegral: u32 = 0x100222c;
    pub const KEY_tintegral: u32 = 0x100222d;
    pub const KEY_because: u32 = 0x1002235;
    pub const KEY_approxeq: u32 = 0x1002248;
    pub const KEY_notapproxeq: u32 = 0x1002247;
    pub const KEY_notidentical: u32 = 0x1002262;
    pub const KEY_stricteq: u32 = 0x1002263;
    pub const KEY_braille_dot_1: u32 = 0xfff1;
    pub const KEY_braille_dot_2: u32 = 0xfff2;
    pub const KEY_braille_dot_3: u32 = 0xfff3;
    pub const KEY_braille_dot_4: u32 = 0xfff4;
    pub const KEY_braille_dot_5: u32 = 0xfff5;
    pub const KEY_braille_dot_6: u32 = 0xfff6;
    pub const KEY_braille_dot_7: u32 = 0xfff7;
    pub const KEY_braille_dot_8: u32 = 0xfff8;
    pub const KEY_braille_dot_9: u32 = 0xfff9;
    pub const KEY_braille_dot_10: u32 = 0xfffa;
    pub const KEY_braille_blank: u32 = 0x1002800;
    pub const KEY_braille_dots_1: u32 = 0x1002801;
    pub const KEY_braille_dots_2: u32 = 0x1002802;
    pub const KEY_braille_dots_12: u32 = 0x1002803;
    pub const KEY_braille_dots_3: u32 = 0x1002804;
    pub const KEY_braille_dots_13: u32 = 0x1002805;
    pub const KEY_braille_dots_23: u32 = 0x1002806;
    pub const KEY_braille_dots_123: u32 = 0x1002807;
    pub const KEY_braille_dots_4: u32 = 0x1002808;
    pub const KEY_braille_dots_14: u32 = 0x1002809;
    pub const KEY_braille_dots_24: u32 = 0x100280a;
    pub const KEY_braille_dots_124: u32 = 0x100280b;
    pub const KEY_braille_dots_34: u32 = 0x100280c;
    pub const KEY_braille_dots_134: u32 = 0x100280d;
    pub const KEY_braille_dots_234: u32 = 0x100280e;
    pub const KEY_braille_dots_1234: u32 = 0x100280f;
    pub const KEY_braille_dots_5: u32 = 0x1002810;
    pub const KEY_braille_dots_15: u32 = 0x1002811;
    pub const KEY_braille_dots_25: u32 = 0x1002812;
    pub const KEY_braille_dots_125: u32 = 0x1002813;
    pub const KEY_braille_dots_35: u32 = 0x1002814;
    pub const KEY_braille_dots_135: u32 = 0x1002815;
    pub const KEY_braille_dots_235: u32 = 0x1002816;
    pub const KEY_braille_dots_1235: u32 = 0x1002817;
    pub const KEY_braille_dots_45: u32 = 0x1002818;
    pub const KEY_braille_dots_145: u32 = 0x1002819;
    pub const KEY_braille_dots_245: u32 = 0x100281a;
    pub const KEY_braille_dots_1245: u32 = 0x100281b;
    pub const KEY_braille_dots_345: u32 = 0x100281c;
    pub const KEY_braille_dots_1345: u32 = 0x100281d;
    pub const KEY_braille_dots_2345: u32 = 0x100281e;
    pub const KEY_braille_dots_12345: u32 = 0x100281f;
    pub const KEY_braille_dots_6: u32 = 0x1002820;
    pub const KEY_braille_dots_16: u32 = 0x1002821;
    pub const KEY_braille_dots_26: u32 = 0x1002822;
    pub const KEY_braille_dots_126: u32 = 0x1002823;
    pub const KEY_braille_dots_36: u32 = 0x1002824;
    pub const KEY_braille_dots_136: u32 = 0x1002825;
    pub const KEY_braille_dots_236: u32 = 0x1002826;
    pub const KEY_braille_dots_1236: u32 = 0x1002827;
    pub const KEY_braille_dots_46: u32 = 0x1002828;
    pub const KEY_braille_dots_146: u32 = 0x1002829;
    pub const KEY_braille_dots_246: u32 = 0x100282a;
    pub const KEY_braille_dots_1246: u32 = 0x100282b;
    pub const KEY_braille_dots_346: u32 = 0x100282c;
    pub const KEY_braille_dots_1346: u32 = 0x100282d;
    pub const KEY_braille_dots_2346: u32 = 0x100282e;
    pub const KEY_braille_dots_12346: u32 = 0x100282f;
    pub const KEY_braille_dots_56: u32 = 0x1002830;
    pub const KEY_braille_dots_156: u32 = 0x1002831;
    pub const KEY_braille_dots_256: u32 = 0x1002832;
    pub const KEY_braille_dots_1256: u32 = 0x1002833;
    pub const KEY_braille_dots_356: u32 = 0x1002834;
    pub const KEY_braille_dots_1356: u32 = 0x1002835;
    pub const KEY_braille_dots_2356: u32 = 0x1002836;
    pub const KEY_braille_dots_12356: u32 = 0x1002837;
    pub const KEY_braille_dots_456: u32 = 0x1002838;
    pub const KEY_braille_dots_1456: u32 = 0x1002839;
    pub const KEY_braille_dots_2456: u32 = 0x100283a;
    pub const KEY_braille_dots_12456: u32 = 0x100283b;
    pub const KEY_braille_dots_3456: u32 = 0x100283c;
    pub const KEY_braille_dots_13456: u32 = 0x100283d;
    pub const KEY_braille_dots_23456: u32 = 0x100283e;
    pub const KEY_braille_dots_123456: u32 = 0x100283f;
    pub const KEY_braille_dots_7: u32 = 0x1002840;
    pub const KEY_braille_dots_17: u32 = 0x1002841;
    pub const KEY_braille_dots_27: u32 = 0x1002842;
    pub const KEY_braille_dots_127: u32 = 0x1002843;
    pub const KEY_braille_dots_37: u32 = 0x1002844;
    pub const KEY_braille_dots_137: u32 = 0x1002845;
    pub const KEY_braille_dots_237: u32 = 0x1002846;
    pub const KEY_braille_dots_1237: u32 = 0x1002847;
    pub const KEY_braille_dots_47: u32 = 0x1002848;
    pub const KEY_braille_dots_147: u32 = 0x1002849;
    pub const KEY_braille_dots_247: u32 = 0x100284a;
    pub const KEY_braille_dots_1247: u32 = 0x100284b;
    pub const KEY_braille_dots_347: u32 = 0x100284c;
    pub const KEY_braille_dots_1347: u32 = 0x100284d;
    pub const KEY_braille_dots_2347: u32 = 0x100284e;
    pub const KEY_braille_dots_12347: u32 = 0x100284f;
    pub const KEY_braille_dots_57: u32 = 0x1002850;
    pub const KEY_braille_dots_157: u32 = 0x1002851;
    pub const KEY_braille_dots_257: u32 = 0x1002852;
    pub const KEY_braille_dots_1257: u32 = 0x1002853;
    pub const KEY_braille_dots_357: u32 = 0x1002854;
    pub const KEY_braille_dots_1357: u32 = 0x1002855;
    pub const KEY_braille_dots_2357: u32 = 0x1002856;
    pub const KEY_braille_dots_12357: u32 = 0x1002857;
    pub const KEY_braille_dots_457: u32 = 0x1002858;
    pub const KEY_braille_dots_1457: u32 = 0x1002859;
    pub const KEY_braille_dots_2457: u32 = 0x100285a;
    pub const KEY_braille_dots_12457: u32 = 0x100285b;
    pub const KEY_braille_dots_3457: u32 = 0x100285c;
    pub const KEY_braille_dots_13457: u32 = 0x100285d;
    pub const KEY_braille_dots_23457: u32 = 0x100285e;
    pub const KEY_braille_dots_123457: u32 = 0x100285f;
    pub const KEY_braille_dots_67: u32 = 0x1002860;
    pub const KEY_braille_dots_167: u32 = 0x1002861;
    pub const KEY_braille_dots_267: u32 = 0x1002862;
    pub const KEY_braille_dots_1267: u32 = 0x1002863;
    pub const KEY_braille_dots_367: u32 = 0x1002864;
    pub const KEY_braille_dots_1367: u32 = 0x1002865;
    pub const KEY_braille_dots_2367: u32 = 0x1002866;
    pub const KEY_braille_dots_12367: u32 = 0x1002867;
    pub const KEY_braille_dots_467: u32 = 0x1002868;
    pub const KEY_braille_dots_1467: u32 = 0x1002869;
    pub const KEY_braille_dots_2467: u32 = 0x100286a;
    pub const KEY_braille_dots_12467: u32 = 0x100286b;
    pub const KEY_braille_dots_3467: u32 = 0x100286c;
    pub const KEY_braille_dots_13467: u32 = 0x100286d;
    pub const KEY_braille_dots_23467: u32 = 0x100286e;
    pub const KEY_braille_dots_123467: u32 = 0x100286f;
    pub const KEY_braille_dots_567: u32 = 0x1002870;
    pub const KEY_braille_dots_1567: u32 = 0x1002871;
    pub const KEY_braille_dots_2567: u32 = 0x1002872;
    pub const KEY_braille_dots_12567: u32 = 0x1002873;
    pub const KEY_braille_dots_3567: u32 = 0x1002874;
    pub const KEY_braille_dots_13567: u32 = 0x1002875;
    pub const KEY_braille_dots_23567: u32 = 0x1002876;
    pub const KEY_braille_dots_123567: u32 = 0x1002877;
    pub const KEY_braille_dots_4567: u32 = 0x1002878;
    pub const KEY_braille_dots_14567: u32 = 0x1002879;
    pub const KEY_braille_dots_24567: u32 = 0x100287a;
    pub const KEY_braille_dots_124567: u32 = 0x100287b;
    pub const KEY_braille_dots_34567: u32 = 0x100287c;
    pub const KEY_braille_dots_134567: u32 = 0x100287d;
    pub const KEY_braille_dots_234567: u32 = 0x100287e;
    pub const KEY_braille_dots_1234567: u32 = 0x100287f;
    pub const KEY_braille_dots_8: u32 = 0x1002880;
    pub const KEY_braille_dots_18: u32 = 0x1002881;
    pub const KEY_braille_dots_28: u32 = 0x1002882;
    pub const KEY_braille_dots_128: u32 = 0x1002883;
    pub const KEY_braille_dots_38: u32 = 0x1002884;
    pub const KEY_braille_dots_138: u32 = 0x1002885;
    pub const KEY_braille_dots_238: u32 = 0x1002886;
    pub const KEY_braille_dots_1238: u32 = 0x1002887;
    pub const KEY_braille_dots_48: u32 = 0x1002888;
    pub const KEY_braille_dots_148: u32 = 0x1002889;
    pub const KEY_braille_dots_248: u32 = 0x100288a;
    pub const KEY_braille_dots_1248: u32 = 0x100288b;
    pub const KEY_braille_dots_348: u32 = 0x100288c;
    pub const KEY_braille_dots_1348: u32 = 0x100288d;
    pub const KEY_braille_dots_2348: u32 = 0x100288e;
    pub const KEY_braille_dots_12348: u32 = 0x100288f;
    pub const KEY_braille_dots_58: u32 = 0x1002890;
    pub const KEY_braille_dots_158: u32 = 0x1002891;
    pub const KEY_braille_dots_258: u32 = 0x1002892;
    pub const KEY_braille_dots_1258: u32 = 0x1002893;
    pub const KEY_braille_dots_358: u32 = 0x1002894;
    pub const KEY_braille_dots_1358: u32 = 0x1002895;
    pub const KEY_braille_dots_2358: u32 = 0x1002896;
    pub const KEY_braille_dots_12358: u32 = 0x1002897;
    pub const KEY_braille_dots_458: u32 = 0x1002898;
    pub const KEY_braille_dots_1458: u32 = 0x1002899;
    pub const KEY_braille_dots_2458: u32 = 0x100289a;
    pub const KEY_braille_dots_12458: u32 = 0x100289b;
    pub const KEY_braille_dots_3458: u32 = 0x100289c;
    pub const KEY_braille_dots_13458: u32 = 0x100289d;
    pub const KEY_braille_dots_23458: u32 = 0x100289e;
    pub const KEY_braille_dots_123458: u32 = 0x100289f;
    pub const KEY_braille_dots_68: u32 = 0x10028a0;
    pub const KEY_braille_dots_168: u32 = 0x10028a1;
    pub const KEY_braille_dots_268: u32 = 0x10028a2;
    pub const KEY_braille_dots_1268: u32 = 0x10028a3;
    pub const KEY_braille_dots_368: u32 = 0x10028a4;
    pub const KEY_braille_dots_1368: u32 = 0x10028a5;
    pub const KEY_braille_dots_2368: u32 = 0x10028a6;
    pub const KEY_braille_dots_12368: u32 = 0x10028a7;
    pub const KEY_braille_dots_468: u32 = 0x10028a8;
    pub const KEY_braille_dots_1468: u32 = 0x10028a9;
    pub const KEY_braille_dots_2468: u32 = 0x10028aa;
    pub const KEY_braille_dots_12468: u32 = 0x10028ab;
    pub const KEY_braille_dots_3468: u32 = 0x10028ac;
    pub const KEY_braille_dots_13468: u32 = 0x10028ad;
    pub const KEY_braille_dots_23468: u32 = 0x10028ae;
    pub const KEY_braille_dots_123468: u32 = 0x10028af;
    pub const KEY_braille_dots_568: u32 = 0x10028b0;
    pub const KEY_braille_dots_1568: u32 = 0x10028b1;
    pub const KEY_braille_dots_2568: u32 = 0x10028b2;
    pub const KEY_braille_dots_12568: u32 = 0x10028b3;
    pub const KEY_braille_dots_3568: u32 = 0x10028b4;
    pub const KEY_braille_dots_13568: u32 = 0x10028b5;
    pub const KEY_braille_dots_23568: u32 = 0x10028b6;
    pub const KEY_braille_dots_123568: u32 = 0x10028b7;
    pub const KEY_braille_dots_4568: u32 = 0x10028b8;
    pub const KEY_braille_dots_14568: u32 = 0x10028b9;
    pub const KEY_braille_dots_24568: u32 = 0x10028ba;
    pub const KEY_braille_dots_124568: u32 = 0x10028bb;
    pub const KEY_braille_dots_34568: u32 = 0x10028bc;
    pub const KEY_braille_dots_134568: u32 = 0x10028bd;
    pub const KEY_braille_dots_234568: u32 = 0x10028be;
    pub const KEY_braille_dots_1234568: u32 = 0x10028bf;
    pub const KEY_braille_dots_78: u32 = 0x10028c0;
    pub const KEY_braille_dots_178: u32 = 0x10028c1;
    pub const KEY_braille_dots_278: u32 = 0x10028c2;
    pub const KEY_braille_dots_1278: u32 = 0x10028c3;
    pub const KEY_braille_dots_378: u32 = 0x10028c4;
    pub const KEY_braille_dots_1378: u32 = 0x10028c5;
    pub const KEY_braille_dots_2378: u32 = 0x10028c6;
    pub const KEY_braille_dots_12378: u32 = 0x10028c7;
    pub const KEY_braille_dots_478: u32 = 0x10028c8;
    pub const KEY_braille_dots_1478: u32 = 0x10028c9;
    pub const KEY_braille_dots_2478: u32 = 0x10028ca;
    pub const KEY_braille_dots_12478: u32 = 0x10028cb;
    pub const KEY_braille_dots_3478: u32 = 0x10028cc;
    pub const KEY_braille_dots_13478: u32 = 0x10028cd;
    pub const KEY_braille_dots_23478: u32 = 0x10028ce;
    pub const KEY_braille_dots_123478: u32 = 0x10028cf;
    pub const KEY_braille_dots_578: u32 = 0x10028d0;
    pub const KEY_braille_dots_1578: u32 = 0x10028d1;
    pub const KEY_braille_dots_2578: u32 = 0x10028d2;
    pub const KEY_braille_dots_12578: u32 = 0x10028d3;
    pub const KEY_braille_dots_3578: u32 = 0x10028d4;
    pub const KEY_braille_dots_13578: u32 = 0x10028d5;
    pub const KEY_braille_dots_23578: u32 = 0x10028d6;
    pub const KEY_braille_dots_123578: u32 = 0x10028d7;
    pub const KEY_braille_dots_4578: u32 = 0x10028d8;
    pub const KEY_braille_dots_14578: u32 = 0x10028d9;
    pub const KEY_braille_dots_24578: u32 = 0x10028da;
    pub const KEY_braille_dots_124578: u32 = 0x10028db;
    pub const KEY_braille_dots_34578: u32 = 0x10028dc;
    pub const KEY_braille_dots_134578: u32 = 0x10028dd;
    pub const KEY_braille_dots_234578: u32 = 0x10028de;
    pub const KEY_braille_dots_1234578: u32 = 0x10028df;
    pub const KEY_braille_dots_678: u32 = 0x10028e0;
    pub const KEY_braille_dots_1678: u32 = 0x10028e1;
    pub const KEY_braille_dots_2678: u32 = 0x10028e2;
    pub const KEY_braille_dots_12678: u32 = 0x10028e3;
    pub const KEY_braille_dots_3678: u32 = 0x10028e4;
    pub const KEY_braille_dots_13678: u32 = 0x10028e5;
    pub const KEY_braille_dots_23678: u32 = 0x10028e6;
    pub const KEY_braille_dots_123678: u32 = 0x10028e7;
    pub const KEY_braille_dots_4678: u32 = 0x10028e8;
    pub const KEY_braille_dots_14678: u32 = 0x10028e9;
    pub const KEY_braille_dots_24678: u32 = 0x10028ea;
    pub const KEY_braille_dots_124678: u32 = 0x10028eb;
    pub const KEY_braille_dots_34678: u32 = 0x10028ec;
    pub const KEY_braille_dots_134678: u32 = 0x10028ed;
    pub const KEY_braille_dots_234678: u32 = 0x10028ee;
    pub const KEY_braille_dots_1234678: u32 = 0x10028ef;
    pub const KEY_braille_dots_5678: u32 = 0x10028f0;
    pub const KEY_braille_dots_15678: u32 = 0x10028f1;
    pub const KEY_braille_dots_25678: u32 = 0x10028f2;
    pub const KEY_braille_dots_125678: u32 = 0x10028f3;
    pub const KEY_braille_dots_35678: u32 = 0x10028f4;
    pub const KEY_braille_dots_135678: u32 = 0x10028f5;
    pub const KEY_braille_dots_235678: u32 = 0x10028f6;
    pub const KEY_braille_dots_1235678: u32 = 0x10028f7;
    pub const KEY_braille_dots_45678: u32 = 0x10028f8;
    pub const KEY_braille_dots_145678: u32 = 0x10028f9;
    pub const KEY_braille_dots_245678: u32 = 0x10028fa;
    pub const KEY_braille_dots_1245678: u32 = 0x10028fb;
    pub const KEY_braille_dots_345678: u32 = 0x10028fc;
    pub const KEY_braille_dots_1345678: u32 = 0x10028fd;
    pub const KEY_braille_dots_2345678: u32 = 0x10028fe;
    pub const KEY_braille_dots_12345678: u32 = 0x10028ff;
    pub const KEY_Sinh_ng: u32 = 0x1000d82;
    pub const KEY_Sinh_h2: u32 = 0x1000d83;
    pub const KEY_Sinh_a: u32 = 0x1000d85;
    pub const KEY_Sinh_aa: u32 = 0x1000d86;
    pub const KEY_Sinh_ae: u32 = 0x1000d87;
    pub const KEY_Sinh_aee: u32 = 0x1000d88;
    pub const KEY_Sinh_i: u32 = 0x1000d89;
    pub const KEY_Sinh_ii: u32 = 0x1000d8a;
    pub const KEY_Sinh_u: u32 = 0x1000d8b;
    pub const KEY_Sinh_uu: u32 = 0x1000d8c;
    pub const KEY_Sinh_ri: u32 = 0x1000d8d;
    pub const KEY_Sinh_rii: u32 = 0x1000d8e;
    pub const KEY_Sinh_lu: u32 = 0x1000d8f;
    pub const KEY_Sinh_luu: u32 = 0x1000d90;
    pub const KEY_Sinh_e: u32 = 0x1000d91;
    pub const KEY_Sinh_ee: u32 = 0x1000d92;
    pub const KEY_Sinh_ai: u32 = 0x1000d93;
    pub const KEY_Sinh_o: u32 = 0x1000d94;
    pub const KEY_Sinh_oo: u32 = 0x1000d95;
    pub const KEY_Sinh_au: u32 = 0x1000d96;
    pub const KEY_Sinh_ka: u32 = 0x1000d9a;
    pub const KEY_Sinh_kha: u32 = 0x1000d9b;
    pub const KEY_Sinh_ga: u32 = 0x1000d9c;
    pub const KEY_Sinh_gha: u32 = 0x1000d9d;
    pub const KEY_Sinh_ng2: u32 = 0x1000d9e;
    pub const KEY_Sinh_nga: u32 = 0x1000d9f;
    pub const KEY_Sinh_ca: u32 = 0x1000da0;
    pub const KEY_Sinh_cha: u32 = 0x1000da1;
    pub const KEY_Sinh_ja: u32 = 0x1000da2;
    pub const KEY_Sinh_jha: u32 = 0x1000da3;
    pub const KEY_Sinh_nya: u32 = 0x1000da4;
    pub const KEY_Sinh_jnya: u32 = 0x1000da5;
    pub const KEY_Sinh_nja: u32 = 0x1000da6;
    pub const KEY_Sinh_tta: u32 = 0x1000da7;
    pub const KEY_Sinh_ttha: u32 = 0x1000da8;
    pub const KEY_Sinh_dda: u32 = 0x1000da9;
    pub const KEY_Sinh_ddha: u32 = 0x1000daa;
    pub const KEY_Sinh_nna: u32 = 0x1000dab;
    pub const KEY_Sinh_ndda: u32 = 0x1000dac;
    pub const KEY_Sinh_tha: u32 = 0x1000dad;
    pub const KEY_Sinh_thha: u32 = 0x1000dae;
    pub const KEY_Sinh_dha: u32 = 0x1000daf;
    pub const KEY_Sinh_dhha: u32 = 0x1000db0;
    pub const KEY_Sinh_na: u32 = 0x1000db1;
    pub const KEY_Sinh_ndha: u32 = 0x1000db3;
    pub const KEY_Sinh_pa: u32 = 0x1000db4;
    pub const KEY_Sinh_pha: u32 = 0x1000db5;
    pub const KEY_Sinh_ba: u32 = 0x1000db6;
    pub const KEY_Sinh_bha: u32 = 0x1000db7;
    pub const KEY_Sinh_ma: u32 = 0x1000db8;
    pub const KEY_Sinh_mba: u32 = 0x1000db9;
    pub const KEY_Sinh_ya: u32 = 0x1000dba;
    pub const KEY_Sinh_ra: u32 = 0x1000dbb;
    pub const KEY_Sinh_la: u32 = 0x1000dbd;
    pub const KEY_Sinh_va: u32 = 0x1000dc0;
    pub const KEY_Sinh_sha: u32 = 0x1000dc1;
    pub const KEY_Sinh_ssha: u32 = 0x1000dc2;
    pub const KEY_Sinh_sa: u32 = 0x1000dc3;
    pub const KEY_Sinh_ha: u32 = 0x1000dc4;
    pub const KEY_Sinh_lla: u32 = 0x1000dc5;
    pub const KEY_Sinh_fa: u32 = 0x1000dc6;
    pub const KEY_Sinh_al: u32 = 0x1000dca;
    pub const KEY_Sinh_aa2: u32 = 0x1000dcf;
    pub const KEY_Sinh_ae2: u32 = 0x1000dd0;
    pub const KEY_Sinh_aee2: u32 = 0x1000dd1;
    pub const KEY_Sinh_i2: u32 = 0x1000dd2;
    pub const KEY_Sinh_ii2: u32 = 0x1000dd3;
    pub const KEY_Sinh_u2: u32 = 0x1000dd4;
    pub const KEY_Sinh_uu2: u32 = 0x1000dd6;
    pub const KEY_Sinh_ru2: u32 = 0x1000dd8;
    pub const KEY_Sinh_e2: u32 = 0x1000dd9;
    pub const KEY_Sinh_ee2: u32 = 0x1000dda;
    pub const KEY_Sinh_ai2: u32 = 0x1000ddb;
    pub const KEY_Sinh_o2: u32 = 0x1000ddc;
    pub const KEY_Sinh_oo2: u32 = 0x1000ddd;
    pub const KEY_Sinh_au2: u32 = 0x1000dde;
    pub const KEY_Sinh_lu2: u32 = 0x1000ddf;
    pub const KEY_Sinh_ruu2: u32 = 0x1000df2;
    pub const KEY_Sinh_luu2: u32 = 0x1000df3;
    pub const KEY_Sinh_kunddaliya: u32 = 0x1000df4;
    pub const KEY_SSHARP: u32 = 0x1001e9e;
    pub const KEY_leftsingleanglequotemark: u32 = 0x1002039;
    pub const KEY_rightsingleanglequotemark: u32 = 0x100203a;
    pub const KEY_XF86ModeLock: u32 = 0x1008ff01;
    pub const KEY_XF86MonBrightnessUp: u32 = 0x1008ff02;
    pub const KEY_XF86MonBrightnessDown: u32 = 0x1008ff03;
    pub const KEY_XF86KbdLightOnOff: u32 = 0x1008ff04;
    pub const KEY_XF86KbdBrightnessUp: u32 = 0x1008ff05;
    pub const KEY_XF86KbdBrightnessDown: u32 = 0x1008ff06;
    pub const KEY_XF86MonBrightnessCycle: u32 = 0x1008ff07;
    pub const KEY_XF86Standby: u32 = 0x1008ff10;
    pub const KEY_XF86AudioLowerVolume: u32 = 0x1008ff11;
    pub const KEY_XF86AudioMute: u32 = 0x1008ff12;
    pub const KEY_XF86AudioRaiseVolume: u32 = 0x1008ff13;
    pub const KEY_XF86AudioPlay: u32 = 0x1008ff14;
    pub const KEY_XF86AudioStop: u32 = 0x1008ff15;
    pub const KEY_XF86AudioPrev: u32 = 0x1008ff16;
    pub const KEY_XF86AudioNext: u32 = 0x1008ff17;
    pub const KEY_XF86HomePage: u32 = 0x1008ff18;
    pub const KEY_XF86Mail: u32 = 0x1008ff19;
    pub const KEY_XF86Start: u32 = 0x1008ff1a;
    pub const KEY_XF86Search: u32 = 0x1008ff1b;
    pub const KEY_XF86AudioRecord: u32 = 0x1008ff1c;
    pub const KEY_XF86Calculator: u32 = 0x1008ff1d;
    pub const KEY_XF86Memo: u32 = 0x1008ff1e;
    pub const KEY_XF86ToDoList: u32 = 0x1008ff1f;
    pub const KEY_XF86Calendar: u32 = 0x1008ff20;
    pub const KEY_XF86PowerDown: u32 = 0x1008ff21;
    pub const KEY_XF86ContrastAdjust: u32 = 0x1008ff22;
    pub const KEY_XF86RockerUp: u32 = 0x1008ff23;
    pub const KEY_XF86RockerDown: u32 = 0x1008ff24;
    pub const KEY_XF86RockerEnter: u32 = 0x1008ff25;
    pub const KEY_XF86Back: u32 = 0x1008ff26;
    pub const KEY_XF86Forward: u32 = 0x1008ff27;
    pub const KEY_XF86Stop: u32 = 0x1008ff28;
    pub const KEY_XF86Refresh: u32 = 0x1008ff29;
    pub const KEY_XF86PowerOff: u32 = 0x1008ff2a;
    pub const KEY_XF86WakeUp: u32 = 0x1008ff2b;
    pub const KEY_XF86Eject: u32 = 0x1008ff2c;
    pub const KEY_XF86ScreenSaver: u32 = 0x1008ff2d;
    pub const KEY_XF86WWW: u32 = 0x1008ff2e;
    pub const KEY_XF86Sleep: u32 = 0x1008ff2f;
    pub const KEY_XF86Favorites: u32 = 0x1008ff30;
    pub const KEY_XF86AudioPause: u32 = 0x1008ff31;
    pub const KEY_XF86AudioMedia: u32 = 0x1008ff32;
    pub const KEY_XF86MyComputer: u32 = 0x1008ff33;
    pub const KEY_XF86VendorHome: u32 = 0x1008ff34;
    pub const KEY_XF86LightBulb: u32 = 0x1008ff35;
    pub const KEY_XF86Shop: u32 = 0x1008ff36;
    pub const KEY_XF86History: u32 = 0x1008ff37;
    pub const KEY_XF86OpenURL: u32 = 0x1008ff38;
    pub const KEY_XF86AddFavorite: u32 = 0x1008ff39;
    pub const KEY_XF86HotLinks: u32 = 0x1008ff3a;
    pub const KEY_XF86BrightnessAdjust: u32 = 0x1008ff3b;
    pub const KEY_XF86Finance: u32 = 0x1008ff3c;
    pub const KEY_XF86Community: u32 = 0x1008ff3d;
    pub const KEY_XF86AudioRewind: u32 = 0x1008ff3e;
    pub const KEY_XF86BackForward: u32 = 0x1008ff3f;
    pub const KEY_XF86Launch0: u32 = 0x1008ff40;
    pub const KEY_XF86Launch1: u32 = 0x1008ff41;
    pub const KEY_XF86Launch2: u32 = 0x1008ff42;
    pub const KEY_XF86Launch3: u32 = 0x1008ff43;
    pub const KEY_XF86Launch4: u32 = 0x1008ff44;
    pub const KEY_XF86Launch5: u32 = 0x1008ff45;
    pub const KEY_XF86Launch6: u32 = 0x1008ff46;
    pub const KEY_XF86Launch7: u32 = 0x1008ff47;
    pub const KEY_XF86Launch8: u32 = 0x1008ff48;
    pub const KEY_XF86Launch9: u32 = 0x1008ff49;
    pub const KEY_XF86LaunchA: u32 = 0x1008ff4a;
    pub const KEY_XF86LaunchB: u32 = 0x1008ff4b;
    pub const KEY_XF86LaunchC: u32 = 0x1008ff4c;
    pub const KEY_XF86LaunchD: u32 = 0x1008ff4d;
    pub const KEY_XF86LaunchE: u32 = 0x1008ff4e;
    pub const KEY_XF86LaunchF: u32 = 0x1008ff4f;
    pub const KEY_XF86ApplicationLeft: u32 = 0x1008ff50;
    pub const KEY_XF86ApplicationRight: u32 = 0x1008ff51;
    pub const KEY_XF86Book: u32 = 0x1008ff52;
    pub const KEY_XF86CD: u32 = 0x1008ff53;
    pub const KEY_XF86MediaSelectCD: u32 = 0x1008ff53;
    pub const KEY_XF86Calculater: u32 = 0x1008ff54;
    pub const KEY_XF86Clear: u32 = 0x1008ff55;
    pub const KEY_XF86Close: u32 = 0x1008ff56;
    pub const KEY_XF86Copy: u32 = 0x1008ff57;
    pub const KEY_XF86Cut: u32 = 0x1008ff58;
    pub const KEY_XF86Display: u32 = 0x1008ff59;
    pub const KEY_XF86DOS: u32 = 0x1008ff5a;
    pub const KEY_XF86Documents: u32 = 0x1008ff5b;
    pub const KEY_XF86Excel: u32 = 0x1008ff5c;
    pub const KEY_XF86Explorer: u32 = 0x1008ff5d;
    pub const KEY_XF86Game: u32 = 0x1008ff5e;
    pub const KEY_XF86Go: u32 = 0x1008ff5f;
    pub const KEY_XF86iTouch: u32 = 0x1008ff60;
    pub const KEY_XF86LogOff: u32 = 0x1008ff61;
    pub const KEY_XF86Market: u32 = 0x1008ff62;
    pub const KEY_XF86Meeting: u32 = 0x1008ff63;
    pub const KEY_XF86MenuKB: u32 = 0x1008ff65;
    pub const KEY_XF86MenuPB: u32 = 0x1008ff66;
    pub const KEY_XF86MySites: u32 = 0x1008ff67;
    pub const KEY_XF86New: u32 = 0x1008ff68;
    pub const KEY_XF86News: u32 = 0x1008ff69;
    pub const KEY_XF86OfficeHome: u32 = 0x1008ff6a;
    pub const KEY_XF86Open: u32 = 0x1008ff6b;
    pub const KEY_XF86Option: u32 = 0x1008ff6c;
    pub const KEY_XF86Paste: u32 = 0x1008ff6d;
    pub const KEY_XF86Phone: u32 = 0x1008ff6e;
    pub const KEY_XF86Q: u32 = 0x1008ff70;
    pub const KEY_XF86Reply: u32 = 0x1008ff72;
    pub const KEY_XF86Reload: u32 = 0x1008ff73;
    pub const KEY_XF86RotateWindows: u32 = 0x1008ff74;
    pub const KEY_XF86RotationPB: u32 = 0x1008ff75;
    pub const KEY_XF86RotationKB: u32 = 0x1008ff76;
    pub const KEY_XF86Save: u32 = 0x1008ff77;
    pub const KEY_XF86ScrollUp: u32 = 0x1008ff78;
    pub const KEY_XF86ScrollDown: u32 = 0x1008ff79;
    pub const KEY_XF86ScrollClick: u32 = 0x1008ff7a;
    pub const KEY_XF86Send: u32 = 0x1008ff7b;
    pub const KEY_XF86Spell: u32 = 0x1008ff7c;
    pub const KEY_XF86SplitScreen: u32 = 0x1008ff7d;
    pub const KEY_XF86Support: u32 = 0x1008ff7e;
    pub const KEY_XF86TaskPane: u32 = 0x1008ff7f;
    pub const KEY_XF86Terminal: u32 = 0x1008ff80;
    pub const KEY_XF86Tools: u32 = 0x1008ff81;
    pub const KEY_XF86Travel: u32 = 0x1008ff82;
    pub const KEY_XF86UserPB: u32 = 0x1008ff84;
    pub const KEY_XF86User1KB: u32 = 0x1008ff85;
    pub const KEY_XF86User2KB: u32 = 0x1008ff86;
    pub const KEY_XF86Video: u32 = 0x1008ff87;
    pub const KEY_XF86WheelButton: u32 = 0x1008ff88;
    pub const KEY_XF86Word: u32 = 0x1008ff89;
    pub const KEY_XF86Xfer: u32 = 0x1008ff8a;
    pub const KEY_XF86ZoomIn: u32 = 0x1008ff8b;
    pub const KEY_XF86ZoomOut: u32 = 0x1008ff8c;
    pub const KEY_XF86Away: u32 = 0x1008ff8d;
    pub const KEY_XF86Messenger: u32 = 0x1008ff8e;
    pub const KEY_XF86WebCam: u32 = 0x1008ff8f;
    pub const KEY_XF86MailForward: u32 = 0x1008ff90;
    pub const KEY_XF86Pictures: u32 = 0x1008ff91;
    pub const KEY_XF86Music: u32 = 0x1008ff92;
    pub const KEY_XF86Battery: u32 = 0x1008ff93;
    pub const KEY_XF86Bluetooth: u32 = 0x1008ff94;
    pub const KEY_XF86WLAN: u32 = 0x1008ff95;
    pub const KEY_XF86UWB: u32 = 0x1008ff96;
    pub const KEY_XF86AudioForward: u32 = 0x1008ff97;
    pub const KEY_XF86AudioRepeat: u32 = 0x1008ff98;
    pub const KEY_XF86AudioRandomPlay: u32 = 0x1008ff99;
    pub const KEY_XF86Subtitle: u32 = 0x1008ff9a;
    pub const KEY_XF86AudioCycleTrack: u32 = 0x1008ff9b;
    pub const KEY_XF86CycleAngle: u32 = 0x1008ff9c;
    pub const KEY_XF86FrameBack: u32 = 0x1008ff9d;
    pub const KEY_XF86FrameForward: u32 = 0x1008ff9e;
    pub const KEY_XF86Time: u32 = 0x1008ff9f;
    pub const KEY_XF86Select: u32 = 0x1008ffa0;
    pub const KEY_XF86View: u32 = 0x1008ffa1;
    pub const KEY_XF86TopMenu: u32 = 0x1008ffa2;
    pub const KEY_XF86Red: u32 = 0x1008ffa3;
    pub const KEY_XF86Green: u32 = 0x1008ffa4;
    pub const KEY_XF86Yellow: u32 = 0x1008ffa5;
    pub const KEY_XF86Blue: u32 = 0x1008ffa6;
    pub const KEY_XF86Suspend: u32 = 0x1008ffa7;
    pub const KEY_XF86Hibernate: u32 = 0x1008ffa8;
    pub const KEY_XF86TouchpadToggle: u32 = 0x1008ffa9;
    pub const KEY_XF86TouchpadOn: u32 = 0x1008ffb0;
    pub const KEY_XF86TouchpadOff: u32 = 0x1008ffb1;
    pub const KEY_XF86AudioMicMute: u32 = 0x1008ffb2;
    pub const KEY_XF86Keyboard: u32 = 0x1008ffb3;
    pub const KEY_XF86WWAN: u32 = 0x1008ffb4;
    pub const KEY_XF86RFKill: u32 = 0x1008ffb5;
    pub const KEY_XF86AudioPreset: u32 = 0x1008ffb6;
    pub const KEY_XF86RotationLockToggle: u32 = 0x1008ffb7;
    pub const KEY_XF86FullScreen: u32 = 0x1008ffb8;
    pub const KEY_XF86Switch_VT_1: u32 = 0x1008fe01;
    pub const KEY_XF86Switch_VT_2: u32 = 0x1008fe02;
    pub const KEY_XF86Switch_VT_3: u32 = 0x1008fe03;
    pub const KEY_XF86Switch_VT_4: u32 = 0x1008fe04;
    pub const KEY_XF86Switch_VT_5: u32 = 0x1008fe05;
    pub const KEY_XF86Switch_VT_6: u32 = 0x1008fe06;
    pub const KEY_XF86Switch_VT_7: u32 = 0x1008fe07;
    pub const KEY_XF86Switch_VT_8: u32 = 0x1008fe08;
    pub const KEY_XF86Switch_VT_9: u32 = 0x1008fe09;
    pub const KEY_XF86Switch_VT_10: u32 = 0x1008fe0a;
    pub const KEY_XF86Switch_VT_11: u32 = 0x1008fe0b;
    pub const KEY_XF86Switch_VT_12: u32 = 0x1008fe0c;
    pub const KEY_XF86Ungrab: u32 = 0x1008fe20;
    pub const KEY_XF86ClearGrab: u32 = 0x1008fe21;
    pub const KEY_XF86Next_VMode: u32 = 0x1008fe22;
    pub const KEY_XF86Prev_VMode: u32 = 0x1008fe23;
    pub const KEY_XF86LogWindowTree: u32 = 0x1008fe24;
    pub const KEY_XF86LogGrabInfo: u32 = 0x1008fe25;
    pub const KEY_XF86MediaPlayPause: u32 = 0x100810a4;
    pub const KEY_XF86Exit: u32 = 0x100810ae;
    pub const KEY_XF86AudioBassBoost: u32 = 0x100810d1;
    pub const KEY_XF86Sport: u32 = 0x100810dc;
    pub const KEY_XF86BrightnessAuto: u32 = 0x100810f4;
    pub const KEY_XF86MonBrightnessAuto: u32 = 0x100810f4;
    pub const KEY_XF86DisplayOff: u32 = 0x100810f5;
    pub const KEY_XF86OK: u32 = 0x10081160;
    pub const KEY_XF86GoTo: u32 = 0x10081162;
    pub const KEY_XF86Info: u32 = 0x10081166;
    pub const KEY_XF86VendorLogo: u32 = 0x10081168;
    pub const KEY_XF86MediaSelectProgramGuide: u32 = 0x1008116a;
    pub const KEY_XF86MediaSelectHome: u32 = 0x1008116e;
    pub const KEY_XF86MediaLanguageMenu: u32 = 0x10081170;
    pub const KEY_XF86MediaTitleMenu: u32 = 0x10081171;
    pub const KEY_XF86AudioChannelMode: u32 = 0x10081175;
    pub const KEY_XF86AspectRatio: u32 = 0x10081177;
    pub const KEY_XF86MediaSelectPC: u32 = 0x10081178;
    pub const KEY_XF86MediaSelectTV: u32 = 0x10081179;
    pub const KEY_XF86MediaSelectCable: u32 = 0x1008117a;
    pub const KEY_XF86MediaSelectVCR: u32 = 0x1008117b;
    pub const KEY_XF86MediaSelectVCRPlus: u32 = 0x1008117c;
    pub const KEY_XF86MediaSelectSatellite: u32 = 0x1008117d;
    pub const KEY_XF86MediaSelectTape: u32 = 0x10081180;
    pub const KEY_XF86MediaSelectRadio: u32 = 0x10081181;
    pub const KEY_XF86MediaSelectTuner: u32 = 0x10081182;
    pub const KEY_XF86MediaPlayer: u32 = 0x10081183;
    pub const KEY_XF86MediaSelectTeletext: u32 = 0x10081184;
    pub const KEY_XF86DVD: u32 = 0x10081185;
    pub const KEY_XF86MediaSelectDVD: u32 = 0x10081185;
    pub const KEY_XF86MediaSelectAuxiliary: u32 = 0x10081186;
    pub const KEY_XF86Audio: u32 = 0x10081188;
    pub const KEY_XF86ChannelUp: u32 = 0x10081192;
    pub const KEY_XF86ChannelDown: u32 = 0x10081193;
    pub const KEY_XF86MediaPlaySlow: u32 = 0x10081199;
    pub const KEY_XF86Break: u32 = 0x1008119b;
    pub const KEY_XF86NumberEntryMode: u32 = 0x1008119d;
    pub const KEY_XF86VideoPhone: u32 = 0x100811a0;
    pub const KEY_XF86ZoomReset: u32 = 0x100811a4;
    pub const KEY_XF86Editor: u32 = 0x100811a6;
    pub const KEY_XF86GraphicsEditor: u32 = 0x100811a8;
    pub const KEY_XF86Presentation: u32 = 0x100811a9;
    pub const KEY_XF86Database: u32 = 0x100811aa;
    pub const KEY_XF86Voicemail: u32 = 0x100811ac;
    pub const KEY_XF86Addressbook: u32 = 0x100811ad;
    pub const KEY_XF86DisplayToggle: u32 = 0x100811af;
    pub const KEY_XF86SpellCheck: u32 = 0x100811b0;
    pub const KEY_XF86ContextMenu: u32 = 0x100811b6;
    pub const KEY_XF86MediaRepeat: u32 = 0x100811b7;
    pub const KEY_XF8610ChannelsUp: u32 = 0x100811b8;
    pub const KEY_XF8610ChannelsDown: u32 = 0x100811b9;
    pub const KEY_XF86Images: u32 = 0x100811ba;
    pub const KEY_XF86NotificationCenter: u32 = 0x100811bc;
    pub const KEY_XF86PickupPhone: u32 = 0x100811bd;
    pub const KEY_XF86HangupPhone: u32 = 0x100811be;
    pub const KEY_XF86LinkPhone: u32 = 0x100811bf;
    pub const KEY_XF86Fn: u32 = 0x100811d0;
    pub const KEY_XF86Fn_Esc: u32 = 0x100811d1;
    pub const KEY_XF86Fn_F1: u32 = 0x100811d2;
    pub const KEY_XF86Fn_F2: u32 = 0x100811d3;
    pub const KEY_XF86Fn_F3: u32 = 0x100811d4;
    pub const KEY_XF86Fn_F4: u32 = 0x100811d5;
    pub const KEY_XF86Fn_F5: u32 = 0x100811d6;
    pub const KEY_XF86Fn_F6: u32 = 0x100811d7;
    pub const KEY_XF86Fn_F7: u32 = 0x100811d8;
    pub const KEY_XF86Fn_F8: u32 = 0x100811d9;
    pub const KEY_XF86Fn_F9: u32 = 0x100811da;
    pub const KEY_XF86Fn_F10: u32 = 0x100811db;
    pub const KEY_XF86Fn_F11: u32 = 0x100811dc;
    pub const KEY_XF86Fn_F12: u32 = 0x100811dd;
    pub const KEY_XF86Fn_1: u32 = 0x100811de;
    pub const KEY_XF86Fn_2: u32 = 0x100811df;
    pub const KEY_XF86Fn_D: u32 = 0x100811e0;
    pub const KEY_XF86Fn_E: u32 = 0x100811e1;
    pub const KEY_XF86Fn_F: u32 = 0x100811e2;
    pub const KEY_XF86Fn_S: u32 = 0x100811e3;
    pub const KEY_XF86Fn_B: u32 = 0x100811e4;
    pub const KEY_XF86FnRightShift: u32 = 0x100811e5;
    pub const KEY_XF86Numeric0: u32 = 0x10081200;
    pub const KEY_XF86Numeric1: u32 = 0x10081201;
    pub const KEY_XF86Numeric2: u32 = 0x10081202;
    pub const KEY_XF86Numeric3: u32 = 0x10081203;
    pub const KEY_XF86Numeric4: u32 = 0x10081204;
    pub const KEY_XF86Numeric5: u32 = 0x10081205;
    pub const KEY_XF86Numeric6: u32 = 0x10081206;
    pub const KEY_XF86Numeric7: u32 = 0x10081207;
    pub const KEY_XF86Numeric8: u32 = 0x10081208;
    pub const KEY_XF86Numeric9: u32 = 0x10081209;
    pub const KEY_XF86NumericStar: u32 = 0x1008120a;
    pub const KEY_XF86NumericPound: u32 = 0x1008120b;
    pub const KEY_XF86NumericA: u32 = 0x1008120c;
    pub const KEY_XF86NumericB: u32 = 0x1008120d;
    pub const KEY_XF86NumericC: u32 = 0x1008120e;
    pub const KEY_XF86NumericD: u32 = 0x1008120f;
    pub const KEY_XF86CameraFocus: u32 = 0x10081210;
    pub const KEY_XF86WPSButton: u32 = 0x10081211;
    pub const KEY_XF86CameraZoomIn: u32 = 0x10081215;
    pub const KEY_XF86CameraZoomOut: u32 = 0x10081216;
    pub const KEY_XF86CameraUp: u32 = 0x10081217;
    pub const KEY_XF86CameraDown: u32 = 0x10081218;
    pub const KEY_XF86CameraLeft: u32 = 0x10081219;
    pub const KEY_XF86CameraRight: u32 = 0x1008121a;
    pub const KEY_XF86AttendantOn: u32 = 0x1008121b;
    pub const KEY_XF86AttendantOff: u32 = 0x1008121c;
    pub const KEY_XF86AttendantToggle: u32 = 0x1008121d;
    pub const KEY_XF86LightsToggle: u32 = 0x1008121e;
    pub const KEY_XF86ALSToggle: u32 = 0x10081230;
    pub const KEY_XF86RefreshRateToggle: u32 = 0x10081232;
    pub const KEY_XF86Buttonconfig: u32 = 0x10081240;
    pub const KEY_XF86Taskmanager: u32 = 0x10081241;
    pub const KEY_XF86Journal: u32 = 0x10081242;
    pub const KEY_XF86ControlPanel: u32 = 0x10081243;
    pub const KEY_XF86AppSelect: u32 = 0x10081244;
    pub const KEY_XF86Screensaver: u32 = 0x10081245;
    pub const KEY_XF86VoiceCommand: u32 = 0x10081246;
    pub const KEY_XF86Assistant: u32 = 0x10081247;
    pub const KEY_XF86EmojiPicker: u32 = 0x10081249;
    pub const KEY_XF86Dictate: u32 = 0x1008124a;
    pub const KEY_XF86CameraAccessEnable: u32 = 0x1008124b;
    pub const KEY_XF86CameraAccessDisable: u32 = 0x1008124c;
    pub const KEY_XF86CameraAccessToggle: u32 = 0x1008124d;
    pub const KEY_XF86Accessibility: u32 = 0x1008124e;
    pub const KEY_XF86DoNotDisturb: u32 = 0x1008124f;
    pub const KEY_XF86BrightnessMin: u32 = 0x10081250;
    pub const KEY_XF86BrightnessMax: u32 = 0x10081251;
    pub const KEY_XF86ElectronicPrivacyScreenOn: u32 = 0x10081252;
    pub const KEY_XF86ElectronicPrivacyScreenOff: u32 = 0x10081253;
    pub const KEY_XF86ActionOnSelection: u32 = 0x10081254;
    pub const KEY_XF86ContextualInsert: u32 = 0x10081255;
    pub const KEY_XF86ContextualQuery: u32 = 0x10081256;
    pub const KEY_XF86KbdInputAssistPrev: u32 = 0x10081260;
    pub const KEY_XF86KbdInputAssistNext: u32 = 0x10081261;
    pub const KEY_XF86KbdInputAssistPrevgroup: u32 = 0x10081262;
    pub const KEY_XF86KbdInputAssistNextgroup: u32 = 0x10081263;
    pub const KEY_XF86KbdInputAssistAccept: u32 = 0x10081264;
    pub const KEY_XF86KbdInputAssistCancel: u32 = 0x10081265;
    pub const KEY_XF86RightUp: u32 = 0x10081266;
    pub const KEY_XF86RightDown: u32 = 0x10081267;
    pub const KEY_XF86LeftUp: u32 = 0x10081268;
    pub const KEY_XF86LeftDown: u32 = 0x10081269;
    pub const KEY_XF86RootMenu: u32 = 0x1008126a;
    pub const KEY_XF86MediaTopMenu: u32 = 0x1008126b;
    pub const KEY_XF86Numeric11: u32 = 0x1008126c;
    pub const KEY_XF86Numeric12: u32 = 0x1008126d;
    pub const KEY_XF86AudioDesc: u32 = 0x1008126e;
    pub const KEY_XF863DMode: u32 = 0x1008126f;
    pub const KEY_XF86NextFavorite: u32 = 0x10081270;
    pub const KEY_XF86StopRecord: u32 = 0x10081271;
    pub const KEY_XF86PauseRecord: u32 = 0x10081272;
    pub const KEY_XF86VOD: u32 = 0x10081273;
    pub const KEY_XF86Unmute: u32 = 0x10081274;
    pub const KEY_XF86FastReverse: u32 = 0x10081275;
    pub const KEY_XF86SlowReverse: u32 = 0x10081276;
    pub const KEY_XF86Data: u32 = 0x10081277;
    pub const KEY_XF86OnScreenKeyboard: u32 = 0x10081278;
    pub const KEY_XF86PrivacyScreenToggle: u32 = 0x10081279;
    pub const KEY_XF86SelectiveScreenshot: u32 = 0x1008127a;
    pub const KEY_XF86NextElement: u32 = 0x1008127b;
    pub const KEY_XF86PreviousElement: u32 = 0x1008127c;
    pub const KEY_XF86AutopilotEngageToggle: u32 = 0x1008127d;
    pub const KEY_XF86MarkWaypoint: u32 = 0x1008127e;
    pub const KEY_XF86Sos: u32 = 0x1008127f;
    pub const KEY_XF86NavChart: u32 = 0x10081280;
    pub const KEY_XF86FishingChart: u32 = 0x10081281;
    pub const KEY_XF86SingleRangeRadar: u32 = 0x10081282;
    pub const KEY_XF86DualRangeRadar: u32 = 0x10081283;
    pub const KEY_XF86RadarOverlay: u32 = 0x10081284;
    pub const KEY_XF86TraditionalSonar: u32 = 0x10081285;
    pub const KEY_XF86ClearvuSonar: u32 = 0x10081286;
    pub const KEY_XF86SidevuSonar: u32 = 0x10081287;
    pub const KEY_XF86NavInfo: u32 = 0x10081288;
    pub const KEY_XF86Macro1: u32 = 0x10081290;
    pub const KEY_XF86Macro2: u32 = 0x10081291;
    pub const KEY_XF86Macro3: u32 = 0x10081292;
    pub const KEY_XF86Macro4: u32 = 0x10081293;
    pub const KEY_XF86Macro5: u32 = 0x10081294;
    pub const KEY_XF86Macro6: u32 = 0x10081295;
    pub const KEY_XF86Macro7: u32 = 0x10081296;
    pub const KEY_XF86Macro8: u32 = 0x10081297;
    pub const KEY_XF86Macro9: u32 = 0x10081298;
    pub const KEY_XF86Macro10: u32 = 0x10081299;
    pub const KEY_XF86Macro11: u32 = 0x1008129a;
    pub const KEY_XF86Macro12: u32 = 0x1008129b;
    pub const KEY_XF86Macro13: u32 = 0x1008129c;
    pub const KEY_XF86Macro14: u32 = 0x1008129d;
    pub const KEY_XF86Macro15: u32 = 0x1008129e;
    pub const KEY_XF86Macro16: u32 = 0x1008129f;
    pub const KEY_XF86Macro17: u32 = 0x100812a0;
    pub const KEY_XF86Macro18: u32 = 0x100812a1;
    pub const KEY_XF86Macro19: u32 = 0x100812a2;
    pub const KEY_XF86Macro20: u32 = 0x100812a3;
    pub const KEY_XF86Macro21: u32 = 0x100812a4;
    pub const KEY_XF86Macro22: u32 = 0x100812a5;
    pub const KEY_XF86Macro23: u32 = 0x100812a6;
    pub const KEY_XF86Macro24: u32 = 0x100812a7;
    pub const KEY_XF86Macro25: u32 = 0x100812a8;
    pub const KEY_XF86Macro26: u32 = 0x100812a9;
    pub const KEY_XF86Macro27: u32 = 0x100812aa;
    pub const KEY_XF86Macro28: u32 = 0x100812ab;
    pub const KEY_XF86Macro29: u32 = 0x100812ac;
    pub const KEY_XF86Macro30: u32 = 0x100812ad;
    pub const KEY_XF86MacroRecordStart: u32 = 0x100812b0;
    pub const KEY_XF86MacroRecordStop: u32 = 0x100812b1;
    pub const KEY_XF86MacroPresetCycle: u32 = 0x100812b2;
    pub const KEY_XF86MacroPreset1: u32 = 0x100812b3;
    pub const KEY_XF86MacroPreset2: u32 = 0x100812b4;
    pub const KEY_XF86MacroPreset3: u32 = 0x100812b5;
    pub const KEY_XF86KbdLcdMenu1: u32 = 0x100812b8;
    pub const KEY_XF86KbdLcdMenu2: u32 = 0x100812b9;
    pub const KEY_XF86KbdLcdMenu3: u32 = 0x100812ba;
    pub const KEY_XF86KbdLcdMenu4: u32 = 0x100812bb;
    pub const KEY_XF86KbdLcdMenu5: u32 = 0x100812bc;
    pub const KEY_XF86PerformanceMode: u32 = 0x100812bd;
    pub const KEY_SunFA_Grave: u32 = 0x1005ff00;
    pub const KEY_SunFA_Circum: u32 = 0x1005ff01;
    pub const KEY_SunFA_Tilde: u32 = 0x1005ff02;
    pub const KEY_SunFA_Acute: u32 = 0x1005ff03;
    pub const KEY_SunFA_Diaeresis: u32 = 0x1005ff04;
    pub const KEY_SunFA_Cedilla: u32 = 0x1005ff05;
    pub const KEY_SunF36: u32 = 0x1005ff10;
    pub const KEY_SunF37: u32 = 0x1005ff11;
    pub const KEY_SunSys_Req: u32 = 0x1005ff60;
    pub const KEY_SunPrint_Screen: u32 = 0x0000ff61;
    pub const KEY_SunCompose: u32 = 0x0000ff20;
    pub const KEY_SunAltGraph: u32 = 0x0000ff7e;
    pub const KEY_SunPageUp: u32 = 0x0000ff55;
    pub const KEY_SunPageDown: u32 = 0x0000ff56;
    pub const KEY_SunUndo: u32 = 0x0000ff65;
    pub const KEY_SunAgain: u32 = 0x0000ff66;
    pub const KEY_SunFind: u32 = 0x0000ff68;
    pub const KEY_SunStop: u32 = 0x0000ff69;
    pub const KEY_SunProps: u32 = 0x1005ff70;
    pub const KEY_SunFront: u32 = 0x1005ff71;
    pub const KEY_SunCopy: u32 = 0x1005ff72;
    pub const KEY_SunOpen: u32 = 0x1005ff73;
    pub const KEY_SunPaste: u32 = 0x1005ff74;
    pub const KEY_SunCut: u32 = 0x1005ff75;
    pub const KEY_SunPowerSwitch: u32 = 0x1005ff76;
    pub const KEY_SunAudioLowerVolume: u32 = 0x1005ff77;
    pub const KEY_SunAudioMute: u32 = 0x1005ff78;
    pub const KEY_SunAudioRaiseVolume: u32 = 0x1005ff79;
    pub const KEY_SunVideoDegauss: u32 = 0x1005ff7a;
    pub const KEY_SunVideoLowerBrightness: u32 = 0x1005ff7b;
    pub const KEY_SunVideoRaiseBrightness: u32 = 0x1005ff7c;
    pub const KEY_SunPowerSwitchShift: u32 = 0x1005ff7d;
    pub const KEY_Dring_accent: u32 = 0x1000feb0;
    pub const KEY_Dcircumflex_accent: u32 = 0x1000fe5e;
    pub const KEY_Dcedilla_accent: u32 = 0x1000fe2c;
    pub const KEY_Dacute_accent: u32 = 0x1000fe27;
    pub const KEY_Dgrave_accent: u32 = 0x1000fe60;
    pub const KEY_Dtilde: u32 = 0x1000fe7e;
    pub const KEY_Ddiaeresis: u32 = 0x1000fe22;
    pub const KEY_DRemove: u32 = 0x1000ff00;
    pub const KEY_hpClearLine: u32 = 0x1000ff6f;
    pub const KEY_hpInsertLine: u32 = 0x1000ff70;
    pub const KEY_hpDeleteLine: u32 = 0x1000ff71;
    pub const KEY_hpInsertChar: u32 = 0x1000ff72;
    pub const KEY_hpDeleteChar: u32 = 0x1000ff73;
    pub const KEY_hpBackTab: u32 = 0x1000ff74;
    pub const KEY_hpKP_BackTab: u32 = 0x1000ff75;
    pub const KEY_hpModelock1: u32 = 0x1000ff48;
    pub const KEY_hpModelock2: u32 = 0x1000ff49;
    pub const KEY_hpReset: u32 = 0x1000ff6c;
    pub const KEY_hpSystem: u32 = 0x1000ff6d;
    pub const KEY_hpUser: u32 = 0x1000ff6e;
    pub const KEY_hpmute_acute: u32 = 0x100000a8;
    pub const KEY_hpmute_grave: u32 = 0x100000a9;
    pub const KEY_hpmute_asciicircum: u32 = 0x100000aa;
    pub const KEY_hpmute_diaeresis: u32 = 0x100000ab;
    pub const KEY_hpmute_asciitilde: u32 = 0x100000ac;
    pub const KEY_hplira: u32 = 0x100000af;
    pub const KEY_hpguilder: u32 = 0x100000be;
    pub const KEY_hpYdiaeresis: u32 = 0x100000ee;
    pub const KEY_hpIO: u32 = 0x100000ee;
    pub const KEY_hplongminus: u32 = 0x100000f6;
    pub const KEY_hpblock: u32 = 0x100000fc;
    pub const KEY_osfCopy: u32 = 0x1004ff02;
    pub const KEY_osfCut: u32 = 0x1004ff03;
    pub const KEY_osfPaste: u32 = 0x1004ff04;
    pub const KEY_osfBackTab: u32 = 0x1004ff07;
    pub const KEY_osfBackSpace: u32 = 0x1004ff08;
    pub const KEY_osfClear: u32 = 0x1004ff0b;
    pub const KEY_osfEscape: u32 = 0x1004ff1b;
    pub const KEY_osfAddMode: u32 = 0x1004ff31;
    pub const KEY_osfPrimaryPaste: u32 = 0x1004ff32;
    pub const KEY_osfQuickPaste: u32 = 0x1004ff33;
    pub const KEY_osfPageLeft: u32 = 0x1004ff40;
    pub const KEY_osfPageUp: u32 = 0x1004ff41;
    pub const KEY_osfPageDown: u32 = 0x1004ff42;
    pub const KEY_osfPageRight: u32 = 0x1004ff43;
    pub const KEY_osfActivate: u32 = 0x1004ff44;
    pub const KEY_osfMenuBar: u32 = 0x1004ff45;
    pub const KEY_osfLeft: u32 = 0x1004ff51;
    pub const KEY_osfUp: u32 = 0x1004ff52;
    pub const KEY_osfRight: u32 = 0x1004ff53;
    pub const KEY_osfDown: u32 = 0x1004ff54;
    pub const KEY_osfEndLine: u32 = 0x1004ff57;
    pub const KEY_osfBeginLine: u32 = 0x1004ff58;
    pub const KEY_osfEndData: u32 = 0x1004ff59;
    pub const KEY_osfBeginData: u32 = 0x1004ff5a;
    pub const KEY_osfPrevMenu: u32 = 0x1004ff5b;
    pub const KEY_osfNextMenu: u32 = 0x1004ff5c;
    pub const KEY_osfPrevField: u32 = 0x1004ff5d;
    pub const KEY_osfNextField: u32 = 0x1004ff5e;
    pub const KEY_osfSelect: u32 = 0x1004ff60;
    pub const KEY_osfInsert: u32 = 0x1004ff63;
    pub const KEY_osfUndo: u32 = 0x1004ff65;
    pub const KEY_osfMenu: u32 = 0x1004ff67;
    pub const KEY_osfCancel: u32 = 0x1004ff69;
    pub const KEY_osfHelp: u32 = 0x1004ff6a;
    pub const KEY_osfSelectAll: u32 = 0x1004ff71;
    pub const KEY_osfDeselectAll: u32 = 0x1004ff72;
    pub const KEY_osfReselect: u32 = 0x1004ff73;
    pub const KEY_osfExtend: u32 = 0x1004ff74;
    pub const KEY_osfRestore: u32 = 0x1004ff78;
    pub const KEY_osfDelete: u32 = 0x1004ffff;
    pub const KEY_Reset: u32 = 0x1000ff6c;
    pub const KEY_System: u32 = 0x1000ff6d;
    pub const KEY_User: u32 = 0x1000ff6e;
    pub const KEY_ClearLine: u32 = 0x1000ff6f;
    pub const KEY_InsertLine: u32 = 0x1000ff70;
    pub const KEY_DeleteLine: u32 = 0x1000ff71;
    pub const KEY_InsertChar: u32 = 0x1000ff72;
    pub const KEY_DeleteChar: u32 = 0x1000ff73;
    pub const KEY_BackTab: u32 = 0x1000ff74;
    pub const KEY_KP_BackTab: u32 = 0x1000ff75;
    pub const KEY_Ext16bit_L: u32 = 0x1000ff76;
    pub const KEY_Ext16bit_R: u32 = 0x1000ff77;
    pub const KEY_mute_acute: u32 = 0x100000a8;
    pub const KEY_mute_grave: u32 = 0x100000a9;
    pub const KEY_mute_asciicircum: u32 = 0x100000aa;
    pub const KEY_mute_diaeresis: u32 = 0x100000ab;
    pub const KEY_mute_asciitilde: u32 = 0x100000ac;
    pub const KEY_lira: u32 = 0x100000af;
    pub const KEY_guilder: u32 = 0x100000be;
    pub const KEY_IO: u32 = 0x100000ee;
    pub const KEY_longminus: u32 = 0x100000f6;
    pub const KEY_block: u32 = 0x100000fc;
}
pub const NAMES: &[(&str, u32)] = &[
    ("0", 0x0030),
    ("1", 0x0031),
    ("2", 0x0032),
    ("3", 0x0033),
    ("3270_AltCursor", 0xfd10),
    ("3270_Attn", 0xfd0e),
    ("3270_BackTab", 0xfd05),
    ("3270_ChangeScreen", 0xfd19),
    ("3270_Copy", 0xfd15),
    ("3270_CursorBlink", 0xfd0f),
    ("3270_CursorSelect", 0xfd1c),
    ("3270_DeleteWord", 0xfd1a),
    ("3270_Duplicate", 0xfd01),
    ("3270_Enter", 0xfd1e),
    ("3270_EraseEOF", 0xfd06),
    ("3270_EraseInput", 0xfd07),
    ("3270_ExSelect", 0xfd1b),
    ("3270_FieldMark", 0xfd02),
    ("3270_Ident", 0xfd13),
    ("3270_Jump", 0xfd12),
    ("3270_KeyClick", 0xfd11),
    ("3270_Left2", 0xfd04),
    ("3270_PA1", 0xfd0a),
    ("3270_PA2", 0xfd0b),
    ("3270_PA3", 0xfd0c),
    ("3270_Play", 0xfd16),
    ("3270_PrintScreen", 0xfd1d),
    ("3270_Quit", 0xfd09),
    ("3270_Record", 0xfd18),
    ("3270_Reset", 0xfd08),
    ("3270_Right2", 0xfd03),
    ("3270_Rule", 0xfd14),
    ("3270_Setup", 0xfd17),
    ("3270_Test", 0xfd0d),
    ("4", 0x0034),
    ("5", 0x0035),
    ("6", 0x0036),
    ("7", 0x0037),
    ("8", 0x0038),
    ("9", 0x0039),
    ("A", 0x0041),
    ("AE", 0x00c6),
    ("Aacute", 0x00c1),
    ("Abelowdot", 0x1001ea0),
    ("Abreve", 0x01c3),
    ("Abreveacute", 0x1001eae),
    ("Abrevebelowdot", 0x1001eb6),
    ("Abrevegrave", 0x1001eb0),
    ("Abrevehook", 0x1001eb2),
    ("Abrevetilde", 0x1001eb4),
    ("AccessX_Enable", 0xfe70),
    ("AccessX_Feedback_Enable", 0xfe71),
    ("Acircumflex", 0x00c2),
    ("Acircumflexacute", 0x1001ea4),
    ("Acircumflexbelowdot", 0x1001eac),
    ("Acircumflexgrave", 0x1001ea6),
    ("Acircumflexhook", 0x1001ea8),
    ("Acircumflextilde", 0x1001eaa),
    ("Adiaeresis", 0x00c4),
    ("Agrave", 0x00c0),
    ("Ahook", 0x1001ea2),
    ("Alt_L", 0xffe9),
    ("Alt_R", 0xffea),
    ("Amacron", 0x03c0),
    ("Aogonek", 0x01a1),
    ("Arabic_0", 0x1000660),
    ("Arabic_1", 0x1000661),
    ("Arabic_2", 0x1000662),
    ("Arabic_3", 0x1000663),
    ("Arabic_4", 0x1000664),
    ("Arabic_5", 0x1000665),
    ("Arabic_6", 0x1000666),
    ("Arabic_7", 0x1000667),
    ("Arabic_8", 0x1000668),
    ("Arabic_9", 0x1000669),
    ("Arabic_ain", 0x05d9),
    ("Arabic_alef", 0x05c7),
    ("Arabic_alefmaksura", 0x05e9),
    ("Arabic_beh", 0x05c8),
    ("Arabic_comma", 0x05ac),
    ("Arabic_dad", 0x05d6),
    ("Arabic_dal", 0x05cf),
    ("Arabic_damma", 0x05ef),
    ("Arabic_dammatan", 0x05ec),
    ("Arabic_ddal", 0x1000688),
    ("Arabic_farsi_yeh", 0x10006cc),
    ("Arabic_fatha", 0x05ee),
    ("Arabic_fathatan", 0x05eb),
    ("Arabic_feh", 0x05e1),
    ("Arabic_fullstop", 0x10006d4),
    ("Arabic_gaf", 0x10006af),
    ("Arabic_ghain", 0x05da),
    ("Arabic_ha", 0x05e7),
    ("Arabic_hah", 0x05cd),
    ("Arabic_hamza", 0x05c1),
    ("Arabic_hamza_above", 0x1000654),
    ("Arabic_hamza_below", 0x1000655),
    ("Arabic_hamzaonalef", 0x05c3),
    ("Arabic_hamzaonwaw", 0x05c4),
    ("Arabic_hamzaonyeh", 0x05c6),
    ("Arabic_hamzaunderalef", 0x05c5),
    ("Arabic_heh", 0x05e7),
    ("Arabic_heh_doachashmee", 0x10006be),
    ("Arabic_heh_goal", 0x10006c1),
    ("Arabic_jeem", 0x05cc),
    ("Arabic_jeh", 0x1000698),
    ("Arabic_kaf", 0x05e3),
    ("Arabic_kasra", 0x05f0),
    ("Arabic_kasratan", 0x05ed),
    ("Arabic_keheh", 0x10006a9),
    ("Arabic_khah", 0x05ce),
    ("Arabic_lam", 0x05e4),
    ("Arabic_madda_above", 0x1000653),
    ("Arabic_maddaonalef", 0x05c2),
    ("Arabic_meem", 0x05e5),
    ("Arabic_noon", 0x05e6),
    ("Arabic_noon_ghunna", 0x10006ba),
    ("Arabic_peh", 0x100067e),
    ("Arabic_percent", 0x100066a),
    ("Arabic_qaf", 0x05e2),
    ("Arabic_question_mark", 0x05bf),
    ("Arabic_ra", 0x05d1),
    ("Arabic_rreh", 0x1000691),
    ("Arabic_sad", 0x05d5),
    ("Arabic_seen", 0x05d3),
    ("Arabic_semicolon", 0x05bb),
    ("Arabic_shadda", 0x05f1),
    ("Arabic_sheen", 0x05d4),
    ("Arabic_sukun", 0x05f2),
    ("Arabic_superscript_alef", 0x1000670),
    ("Arabic_switch", 0xff7e),
    ("Arabic_tah", 0x05d7),
    ("Arabic_tatweel", 0x05e0),
    ("Arabic_tcheh", 0x1000686),
    ("Arabic_teh", 0x05ca),
    ("Arabic_tehmarbuta", 0x05c9),
    ("Arabic_thal", 0x05d0),
    ("Arabic_theh", 0x05cb),
    ("Arabic_tteh", 0x1000679),
    ("Arabic_veh", 0x10006a4),
    ("Arabic_waw", 0x05e8),
    ("Arabic_yeh", 0x05ea),
    ("Arabic_yeh_baree", 0x10006d2),
    ("Arabic_zah", 0x05d8),
    ("Arabic_zain", 0x05d2),
    ("Aring", 0x00c5),
    ("Armenian_AT", 0x1000538),
    ("Armenian_AYB", 0x1000531),
    ("Armenian_BEN", 0x1000532),
    ("Armenian_CHA", 0x1000549),
    ("Armenian_DA", 0x1000534),
    ("Armenian_DZA", 0x1000541),
    ("Armenian_E", 0x1000537),
    ("Armenian_FE", 0x1000556),
    ("Armenian_GHAT", 0x1000542),
    ("Armenian_GIM", 0x1000533),
    ("Armenian_HI", 0x1000545),
    ("Armenian_HO", 0x1000540),
    ("Armenian_INI", 0x100053b),
    ("Armenian_JE", 0x100054b),
    ("Armenian_KE", 0x1000554),
    ("Armenian_KEN", 0x100053f),
    ("Armenian_KHE", 0x100053d),
    ("Armenian_LYUN", 0x100053c),
    ("Armenian_MEN", 0x1000544),
    ("Armenian_NU", 0x1000546),
    ("Armenian_O", 0x1000555),
    ("Armenian_PE", 0x100054a),
    ("Armenian_PYUR", 0x1000553),
    ("Armenian_RA", 0x100054c),
    ("Armenian_RE", 0x1000550),
    ("Armenian_SE", 0x100054d),
    ("Armenian_SHA", 0x1000547),
    ("Armenian_TCHE", 0x1000543),
    ("Armenian_TO", 0x1000539),
    ("Armenian_TSA", 0x100053e),
    ("Armenian_TSO", 0x1000551),
    ("Armenian_TYUN", 0x100054f),
    ("Armenian_VEV", 0x100054e),
    ("Armenian_VO", 0x1000548),
    ("Armenian_VYUN", 0x1000552),
    ("Armenian_YECH", 0x1000535),
    ("Armenian_ZA", 0x1000536),
    ("Armenian_ZHE", 0x100053a),
    ("Armenian_accent", 0x100055b),
    ("Armenian_amanak", 0x100055c),
    ("Armenian_apostrophe", 0x100055a),
    ("Armenian_at", 0x1000568),
    ("Armenian_ayb", 0x1000561),
    ("Armenian_ben", 0x1000562),
    ("Armenian_but", 0x100055d),
    ("Armenian_cha", 0x1000579),
    ("Armenian_da", 0x1000564),
    ("Armenian_dza", 0x1000571),
    ("Armenian_e", 0x1000567),
    ("Armenian_exclam", 0x100055c),
    ("Armenian_fe", 0x1000586),
    ("Armenian_full_stop", 0x1000589),
    ("Armenian_ghat", 0x1000572),
    ("Armenian_gim", 0x1000563),
    ("Armenian_hi", 0x1000575),
    ("Armenian_ho", 0x1000570),
    ("Armenian_hyphen", 0x100058a),
    ("Armenian_ini", 0x100056b),
    ("Armenian_je", 0x100057b),
    ("Armenian_ke", 0x1000584),
    ("Armenian_ken", 0x100056f),
    ("Armenian_khe", 0x100056d),
    ("Armenian_ligature_ew", 0x1000587),
    ("Armenian_lyun", 0x100056c),
    ("Armenian_men", 0x1000574),
    ("Armenian_nu", 0x1000576),
    ("Armenian_o", 0x1000585),
    ("Armenian_paruyk", 0x100055e),
    ("Armenian_pe", 0x100057a),
    ("Armenian_pyur", 0x1000583),
    ("Armenian_question", 0x100055e),
    ("Armenian_ra", 0x100057c),
    ("Armenian_re", 0x1000580),
    ("Armenian_se", 0x100057d),
    ("Armenian_separation_mark", 0x100055d),
    ("Armenian_sha", 0x1000577),
    ("Armenian_shesht", 0x100055b),
    ("Armenian_tche", 0x1000573),
    ("Armenian_to", 0x1000569),
    ("Armenian_tsa", 0x100056e),
    ("Armenian_tso", 0x1000581),
    ("Armenian_tyun", 0x100057f),
    ("Armenian_verjaket", 0x1000589),
    ("Armenian_vev", 0x100057e),
    ("Armenian_vo", 0x1000578),
    ("Armenian_vyun", 0x1000582),
    ("Armenian_yech", 0x1000565),
    ("Armenian_yentamna", 0x100058a),
    ("Armenian_za", 0x1000566),
    ("Armenian_zhe", 0x100056a),
    ("Atilde", 0x00c3),
    ("AudibleBell_Enable", 0xfe7a),
    ("B", 0x0042),
    ("Babovedot", 0x1001e02),
    ("BackSpace", 0xff08),
    ("BackTab", 0x1000ff74),
    ("Begin", 0xff58),
    ("BounceKeys_Enable", 0xfe74),
    ("Break", 0xff6b),
    ("Byelorussian_SHORTU", 0x06be),
    ("Byelorussian_shortu", 0x06ae),
    ("C", 0x0043),
    ("CH", 0xfea2),
    ("C_H", 0xfea5),
    ("C_h", 0xfea4),
    ("Cabovedot", 0x02c5),
    ("Cacute", 0x01c6),
    ("Cancel", 0xff69),
    ("Caps_Lock", 0xffe5),
    ("Ccaron", 0x01c8),
    ("Ccedilla", 0x00c7),
    ("Ccircumflex", 0x02c6),
    ("Ch", 0xfea1),
    ("Clear", 0xff0b),
    ("ClearLine", 0x1000ff6f),
    ("Codeinput", 0xff37),
    ("ColonSign", 0x10020a1),
    ("Control_L", 0xffe3),
    ("Control_R", 0xffe4),
    ("CruzeiroSign", 0x10020a2),
    ("Cyrillic_A", 0x06e1),
    ("Cyrillic_BE", 0x06e2),
    ("Cyrillic_CHE", 0x06fe),
    ("Cyrillic_CHE_descender", 0x10004b6),
    ("Cyrillic_CHE_vertstroke", 0x10004b8),
    ("Cyrillic_DE", 0x06e4),
    ("Cyrillic_DZHE", 0x06bf),
    ("Cyrillic_E", 0x06fc),
    ("Cyrillic_EF", 0x06e6),
    ("Cyrillic_EL", 0x06ec),
    ("Cyrillic_EM", 0x06ed),
    ("Cyrillic_EN", 0x06ee),
    ("Cyrillic_EN_descender", 0x10004a2),
    ("Cyrillic_ER", 0x06f2),
    ("Cyrillic_ES", 0x06f3),
    ("Cyrillic_GHE", 0x06e7),
    ("Cyrillic_GHE_bar", 0x1000492),
    ("Cyrillic_HA", 0x06e8),
    ("Cyrillic_HARDSIGN", 0x06ff),
    ("Cyrillic_HA_descender", 0x10004b2),
    ("Cyrillic_I", 0x06e9),
    ("Cyrillic_IE", 0x06e5),
    ("Cyrillic_IO", 0x06b3),
    ("Cyrillic_I_macron", 0x10004e2),
    ("Cyrillic_JE", 0x06b8),
    ("Cyrillic_KA", 0x06eb),
    ("Cyrillic_KA_descender", 0x100049a),
    ("Cyrillic_KA_vertstroke", 0x100049c),
    ("Cyrillic_LJE", 0x06b9),
    ("Cyrillic_NJE", 0x06ba),
    ("Cyrillic_O", 0x06ef),
    ("Cyrillic_O_bar", 0x10004e8),
    ("Cyrillic_PE", 0x06f0),
    ("Cyrillic_SCHWA", 0x10004d8),
    ("Cyrillic_SHA", 0x06fb),
    ("Cyrillic_SHCHA", 0x06fd),
    ("Cyrillic_SHHA", 0x10004ba),
    ("Cyrillic_SHORTI", 0x06ea),
    ("Cyrillic_SOFTSIGN", 0x06f8),
    ("Cyrillic_TE", 0x06f4),
    ("Cyrillic_TSE", 0x06e3),
    ("Cyrillic_U", 0x06f5),
    ("Cyrillic_U_macron", 0x10004ee),
    ("Cyrillic_U_straight", 0x10004ae),
    ("Cyrillic_U_straight_bar", 0x10004b0),
    ("Cyrillic_VE", 0x06f7),
    ("Cyrillic_YA", 0x06f1),
    ("Cyrillic_YERU", 0x06f9),
    ("Cyrillic_YU", 0x06e0),
    ("Cyrillic_ZE", 0x06fa),
    ("Cyrillic_ZHE", 0x06f6),
    ("Cyrillic_ZHE_descender", 0x1000496),
    ("Cyrillic_a", 0x06c1),
    ("Cyrillic_be", 0x06c2),
    ("Cyrillic_che", 0x06de),
    ("Cyrillic_che_descender", 0x10004b7),
    ("Cyrillic_che_vertstroke", 0x10004b9),
    ("Cyrillic_de", 0x06c4),
    ("Cyrillic_dzhe", 0x06af),
    ("Cyrillic_e", 0x06dc),
    ("Cyrillic_ef", 0x06c6),
    ("Cyrillic_el", 0x06cc),
    ("Cyrillic_em", 0x06cd),
    ("Cyrillic_en", 0x06ce),
    ("Cyrillic_en_descender", 0x10004a3),
    ("Cyrillic_er", 0x06d2),
    ("Cyrillic_es", 0x06d3),
    ("Cyrillic_ghe", 0x06c7),
    ("Cyrillic_ghe_bar", 0x1000493),
    ("Cyrillic_ha", 0x06c8),
    ("Cyrillic_ha_descender", 0x10004b3),
    ("Cyrillic_hardsign", 0x06df),
    ("Cyrillic_i", 0x06c9),
    ("Cyrillic_i_macron", 0x10004e3),
    ("Cyrillic_ie", 0x06c5),
    ("Cyrillic_io", 0x06a3),
    ("Cyrillic_je", 0x06a8),
    ("Cyrillic_ka", 0x06cb),
    ("Cyrillic_ka_descender", 0x100049b),
    ("Cyrillic_ka_vertstroke", 0x100049d),
    ("Cyrillic_lje", 0x06a9),
    ("Cyrillic_nje", 0x06aa),
    ("Cyrillic_o", 0x06cf),
    ("Cyrillic_o_bar", 0x10004e9),
    ("Cyrillic_pe", 0x06d0),
    ("Cyrillic_schwa", 0x10004d9),
    ("Cyrillic_sha", 0x06db),
    ("Cyrillic_shcha", 0x06dd),
    ("Cyrillic_shha", 0x10004bb),
    ("Cyrillic_shorti", 0x06ca),
    ("Cyrillic_softsign", 0x06d8),
    ("Cyrillic_te", 0x06d4),
    ("Cyrillic_tse", 0x06c3),
    ("Cyrillic_u", 0x06d5),
    ("Cyrillic_u_macron", 0x10004ef),
    ("Cyrillic_u_straight", 0x10004af),
    ("Cyrillic_u_straight_bar", 0x10004b1),
    ("Cyrillic_ve", 0x06d7),
    ("Cyrillic_ya", 0x06d1),
    ("Cyrillic_yeru", 0x06d9),
    ("Cyrillic_yu", 0x06c0),
    ("Cyrillic_ze", 0x06da),
    ("Cyrillic_zhe", 0x06d6),
    ("Cyrillic_zhe_descender", 0x1000497),
    ("D", 0x0044),
    ("DRemove", 0x1000ff00),
    ("Dabovedot", 0x1001e0a),
    ("Dacute_accent", 0x1000fe27),
    ("Dcaron", 0x01cf),
    ("Dcedilla_accent", 0x1000fe2c),
    ("Dcircumflex_accent", 0x1000fe5e),
    ("Ddiaeresis", 0x1000fe22),
    ("Delete", 0xffff),
    ("DeleteChar", 0x1000ff73),
    ("DeleteLine", 0x1000ff71),
    ("Dgrave_accent", 0x1000fe60),
    ("DongSign", 0x10020ab),
    ("Down", 0xff54),
    ("Dring_accent", 0x1000feb0),
    ("Dstroke", 0x01d0),
    ("Dtilde", 0x1000fe7e),
    ("E", 0x0045),
    ("ENG", 0x03bd),
    ("ETH", 0x00d0),
    ("EZH", 0x10001b7),
    ("Eabovedot", 0x03cc),
    ("Eacute", 0x00c9),
    ("Ebelowdot", 0x1001eb8),
    ("Ecaron", 0x01cc),
    ("Ecircumflex", 0x00ca),
    ("Ecircumflexacute", 0x1001ebe),
    ("Ecircumflexbelowdot", 0x1001ec6),
    ("Ecircumflexgrave", 0x1001ec0),
    ("Ecircumflexhook", 0x1001ec2),
    ("Ecircumflextilde", 0x1001ec4),
    ("EcuSign", 0x10020a0),
    ("Ediaeresis", 0x00cb),
    ("Egrave", 0x00c8),
    ("Ehook", 0x1001eba),
    ("Eisu_Shift", 0xff2f),
    ("Eisu_toggle", 0xff30),
    ("Emacron", 0x03aa),
    ("End", 0xff57),
    ("Eogonek", 0x01ca),
    ("Escape", 0xff1b),
    ("Eth", 0x00d0),
    ("Etilde", 0x1001ebc),
    ("EuroSign", 0x20ac),
    ("Execute", 0xff62),
    ("Ext16bit_L", 0x1000ff76),
    ("Ext16bit_R", 0x1000ff77),
    ("F", 0x0046),
    ("F1", 0xffbe),
    ("F10", 0xffc7),
    ("F11", 0xffc8),
    ("F12", 0xffc9),
    ("F13", 0xffca),
    ("F14", 0xffcb),
    ("F15", 0xffcc),
    ("F16", 0xffcd),
    ("F17", 0xffce),
    ("F18", 0xffcf),
    ("F19", 0xffd0),
    ("F2", 0xffbf),
    ("F20", 0xffd1),
    ("F21", 0xffd2),
    ("F22", 0xffd3),
    ("F23", 0xffd4),
    ("F24", 0xffd5),
    ("F25", 0xffd6),
    ("F26", 0xffd7),
    ("F27", 0xffd8),
    ("F28", 0xffd9),
    ("F29", 0xffda),
    ("F3", 0xffc0),
    ("F30", 0xffdb),
    ("F31", 0xffdc),
    ("F32", 0xffdd),
    ("F33", 0xffde),
    ("F34", 0xffdf),
    ("F35", 0xffe0),
    ("F4", 0xffc1),
    ("F5", 0xffc2),
    ("F6", 0xffc3),
    ("F7", 0xffc4),
    ("F8", 0xffc5),
    ("F9", 0xffc6),
    ("FFrancSign", 0x10020a3),
    ("Fabovedot", 0x1001e1e),
    ("Farsi_0", 0x10006f0),
    ("Farsi_1", 0x10006f1),
    ("Farsi_2", 0x10006f2),
    ("Farsi_3", 0x10006f3),
    ("Farsi_4", 0x10006f4),
    ("Farsi_5", 0x10006f5),
    ("Farsi_6", 0x10006f6),
    ("Farsi_7", 0x10006f7),
    ("Farsi_8", 0x10006f8),
    ("Farsi_9", 0x10006f9),
    ("Farsi_yeh", 0x10006cc),
    ("Find", 0xff68),
    ("First_Virtual_Screen", 0xfed0),
    ("G", 0x0047),
    ("Gabovedot", 0x02d5),
    ("Gbreve", 0x02ab),
    ("Gcaron", 0x10001e6),
    ("Gcedilla", 0x03ab),
    ("Gcircumflex", 0x02d8),
    ("Georgian_an", 0x10010d0),
    ("Georgian_ban", 0x10010d1),
    ("Georgian_can", 0x10010ea),
    ("Georgian_char", 0x10010ed),
    ("Georgian_chin", 0x10010e9),
    ("Georgian_cil", 0x10010ec),
    ("Georgian_don", 0x10010d3),
    ("Georgian_en", 0x10010d4),
    ("Georgian_fi", 0x10010f6),
    ("Georgian_gan", 0x10010d2),
    ("Georgian_ghan", 0x10010e6),
    ("Georgian_hae", 0x10010f0),
    ("Georgian_har", 0x10010f4),
    ("Georgian_he", 0x10010f1),
    ("Georgian_hie", 0x10010f2),
    ("Georgian_hoe", 0x10010f5),
    ("Georgian_in", 0x10010d8),
    ("Georgian_jhan", 0x10010ef),
    ("Georgian_jil", 0x10010eb),
    ("Georgian_kan", 0x10010d9),
    ("Georgian_khar", 0x10010e5),
    ("Georgian_las", 0x10010da),
    ("Georgian_man", 0x10010db),
    ("Georgian_nar", 0x10010dc),
    ("Georgian_on", 0x10010dd),
    ("Georgian_par", 0x10010de),
    ("Georgian_phar", 0x10010e4),
    ("Georgian_qar", 0x10010e7),
    ("Georgian_rae", 0x10010e0),
    ("Georgian_san", 0x10010e1),
    ("Georgian_shin", 0x10010e8),
    ("Georgian_tan", 0x10010d7),
    ("Georgian_tar", 0x10010e2),
    ("Georgian_un", 0x10010e3),
    ("Georgian_vin", 0x10010d5),
    ("Georgian_we", 0x10010f3),
    ("Georgian_xan", 0x10010ee),
    ("Georgian_zen", 0x10010d6),
    ("Georgian_zhar", 0x10010df),
    ("Greek_ALPHA", 0x07c1),
    ("Greek_ALPHAaccent", 0x07a1),
    ("Greek_BETA", 0x07c2),
    ("Greek_CHI", 0x07d7),
    ("Greek_DELTA", 0x07c4),
    ("Greek_EPSILON", 0x07c5),
    ("Greek_EPSILONaccent", 0x07a2),
    ("Greek_ETA", 0x07c7),
    ("Greek_ETAaccent", 0x07a3),
    ("Greek_GAMMA", 0x07c3),
    ("Greek_IOTA", 0x07c9),
    ("Greek_IOTAaccent", 0x07a4),
    ("Greek_IOTAdiaeresis", 0x07a5),
    ("Greek_IOTAdieresis", 0x07a5),
    ("Greek_KAPPA", 0x07ca),
    ("Greek_LAMBDA", 0x07cb),
    ("Greek_LAMDA", 0x07cb),
    ("Greek_MU", 0x07cc),
    ("Greek_NU", 0x07cd),
    ("Greek_OMEGA", 0x07d9),
    ("Greek_OMEGAaccent", 0x07ab),
    ("Greek_OMICRON", 0x07cf),
    ("Greek_OMICRONaccent", 0x07a7),
    ("Greek_PHI", 0x07d6),
    ("Greek_PI", 0x07d0),
    ("Greek_PSI", 0x07d8),
    ("Greek_RHO", 0x07d1),
    ("Greek_SIGMA", 0x07d2),
    ("Greek_TAU", 0x07d4),
    ("Greek_THETA", 0x07c8),
    ("Greek_UPSILON", 0x07d5),
    ("Greek_UPSILONaccent", 0x07a8),
    ("Greek_UPSILONdieresis", 0x07a9),
    ("Greek_XI", 0x07ce),
    ("Greek_ZETA", 0x07c6),
    ("Greek_accentdieresis", 0x07ae),
    ("Greek_alpha", 0x07e1),
    ("Greek_alphaaccent", 0x07b1),
    ("Greek_beta", 0x07e2),
    ("Greek_chi", 0x07f7),
    ("Greek_delta", 0x07e4),
    ("Greek_epsilon", 0x07e5),
    ("Greek_epsilonaccent", 0x07b2),
    ("Greek_eta", 0x07e7),
    ("Greek_etaaccent", 0x07b3),
    ("Greek_finalsmallsigma", 0x07f3),
    ("Greek_gamma", 0x07e3),
    ("Greek_horizbar", 0x07af),
    ("Greek_iota", 0x07e9),
    ("Greek_iotaaccent", 0x07b4),
    ("Greek_iotaaccentdieresis", 0x07b6),
    ("Greek_iotadieresis", 0x07b5),
    ("Greek_kappa", 0x07ea),
    ("Greek_lambda", 0x07eb),
    ("Greek_lamda", 0x07eb),
    ("Greek_mu", 0x07ec),
    ("Greek_nu", 0x07ed),
    ("Greek_omega", 0x07f9),
    ("Greek_omegaaccent", 0x07bb),
    ("Greek_omicron", 0x07ef),
    ("Greek_omicronaccent", 0x07b7),
    ("Greek_phi", 0x07f6),
    ("Greek_pi", 0x07f0),
    ("Greek_psi", 0x07f8),
    ("Greek_rho", 0x07f1),
    ("Greek_sigma", 0x07f2),
    ("Greek_switch", 0xff7e),
    ("Greek_tau", 0x07f4),
    ("Greek_theta", 0x07e8),
    ("Greek_upsilon", 0x07f5),
    ("Greek_upsilonaccent", 0x07b8),
    ("Greek_upsilonaccentdieresis", 0x07ba),
    ("Greek_upsilondieresis", 0x07b9),
    ("Greek_xi", 0x07ee),
    ("Greek_zeta", 0x07e6),
    ("H", 0x0048),
    ("Hangul", 0xff31),
    ("Hangul_A", 0x0ebf),
    ("Hangul_AE", 0x0ec0),
    ("Hangul_AraeA", 0x0ef6),
    ("Hangul_AraeAE", 0x0ef7),
    ("Hangul_Banja", 0xff39),
    ("Hangul_Cieuc", 0x0eba),
    ("Hangul_Codeinput", 0xff37),
    ("Hangul_Dikeud", 0x0ea7),
    ("Hangul_E", 0x0ec4),
    ("Hangul_EO", 0x0ec3),
    ("Hangul_EU", 0x0ed1),
    ("Hangul_End", 0xff33),
    ("Hangul_Hanja", 0xff34),
    ("Hangul_Hieuh", 0x0ebe),
    ("Hangul_I", 0x0ed3),
    ("Hangul_Ieung", 0x0eb7),
    ("Hangul_J_Cieuc", 0x0eea),
    ("Hangul_J_Dikeud", 0x0eda),
    ("Hangul_J_Hieuh", 0x0eee),
    ("Hangul_J_Ieung", 0x0ee8),
    ("Hangul_J_Jieuj", 0x0ee9),
    ("Hangul_J_Khieuq", 0x0eeb),
    ("Hangul_J_Kiyeog", 0x0ed4),
    ("Hangul_J_KiyeogSios", 0x0ed6),
    ("Hangul_J_KkogjiDalrinIeung", 0x0ef9),
    ("Hangul_J_Mieum", 0x0ee3),
    ("Hangul_J_Nieun", 0x0ed7),
    ("Hangul_J_NieunHieuh", 0x0ed9),
    ("Hangul_J_NieunJieuj", 0x0ed8),
    ("Hangul_J_PanSios", 0x0ef8),
    ("Hangul_J_Phieuf", 0x0eed),
    ("Hangul_J_Pieub", 0x0ee4),
    ("Hangul_J_PieubSios", 0x0ee5),
    ("Hangul_J_Rieul", 0x0edb),
    ("Hangul_J_RieulHieuh", 0x0ee2),
    ("Hangul_J_RieulKiyeog", 0x0edc),
    ("Hangul_J_RieulMieum", 0x0edd),
    ("Hangul_J_RieulPhieuf", 0x0ee1),
    ("Hangul_J_RieulPieub", 0x0ede),
    ("Hangul_J_RieulSios", 0x0edf),
    ("Hangul_J_RieulTieut", 0x0ee0),
    ("Hangul_J_Sios", 0x0ee6),
    ("Hangul_J_SsangKiyeog", 0x0ed5),
    ("Hangul_J_SsangSios", 0x0ee7),
    ("Hangul_J_Tieut", 0x0eec),
    ("Hangul_J_YeorinHieuh", 0x0efa),
    ("Hangul_Jamo", 0xff35),
    ("Hangul_Jeonja", 0xff38),
    ("Hangul_Jieuj", 0x0eb8),
    ("Hangul_Khieuq", 0x0ebb),
    ("Hangul_Kiyeog", 0x0ea1),
    ("Hangul_KiyeogSios", 0x0ea3),
    ("Hangul_KkogjiDalrinIeung", 0x0ef3),
    ("Hangul_Mieum", 0x0eb1),
    ("Hangul_MultipleCandidate", 0xff3d),
    ("Hangul_Nieun", 0x0ea4),
    ("Hangul_NieunHieuh", 0x0ea6),
    ("Hangul_NieunJieuj", 0x0ea5),
    ("Hangul_O", 0x0ec7),
    ("Hangul_OE", 0x0eca),
    ("Hangul_PanSios", 0x0ef2),
    ("Hangul_Phieuf", 0x0ebd),
    ("Hangul_Pieub", 0x0eb2),
    ("Hangul_PieubSios", 0x0eb4),
    ("Hangul_PostHanja", 0xff3b),
    ("Hangul_PreHanja", 0xff3a),
    ("Hangul_PreviousCandidate", 0xff3e),
    ("Hangul_Rieul", 0x0ea9),
    ("Hangul_RieulHieuh", 0x0eb0),
    ("Hangul_RieulKiyeog", 0x0eaa),
    ("Hangul_RieulMieum", 0x0eab),
    ("Hangul_RieulPhieuf", 0x0eaf),
    ("Hangul_RieulPieub", 0x0eac),
    ("Hangul_RieulSios", 0x0ead),
    ("Hangul_RieulTieut", 0x0eae),
    ("Hangul_RieulYeorinHieuh", 0x0eef),
    ("Hangul_Romaja", 0xff36),
    ("Hangul_SingleCandidate", 0xff3c),
    ("Hangul_Sios", 0x0eb5),
    ("Hangul_Special", 0xff3f),
    ("Hangul_SsangDikeud", 0x0ea8),
    ("Hangul_SsangJieuj", 0x0eb9),
    ("Hangul_SsangKiyeog", 0x0ea2),
    ("Hangul_SsangPieub", 0x0eb3),
    ("Hangul_SsangSios", 0x0eb6),
    ("Hangul_Start", 0xff32),
    ("Hangul_SunkyeongeumMieum", 0x0ef0),
    ("Hangul_SunkyeongeumPhieuf", 0x0ef4),
    ("Hangul_SunkyeongeumPieub", 0x0ef1),
    ("Hangul_Tieut", 0x0ebc),
    ("Hangul_U", 0x0ecc),
    ("Hangul_WA", 0x0ec8),
    ("Hangul_WAE", 0x0ec9),
    ("Hangul_WE", 0x0ece),
    ("Hangul_WEO", 0x0ecd),
    ("Hangul_WI", 0x0ecf),
    ("Hangul_YA", 0x0ec1),
    ("Hangul_YAE", 0x0ec2),
    ("Hangul_YE", 0x0ec6),
    ("Hangul_YEO", 0x0ec5),
    ("Hangul_YI", 0x0ed2),
    ("Hangul_YO", 0x0ecb),
    ("Hangul_YU", 0x0ed0),
    ("Hangul_YeorinHieuh", 0x0ef5),
    ("Hangul_switch", 0xff7e),
    ("Hankaku", 0xff29),
    ("Hcircumflex", 0x02a6),
    ("Hebrew_switch", 0xff7e),
    ("Help", 0xff6a),
    ("Henkan", 0xff23),
    ("Henkan_Mode", 0xff23),
    ("Hiragana", 0xff25),
    ("Hiragana_Katakana", 0xff27),
    ("Home", 0xff50),
    ("Hstroke", 0x02a1),
    ("Hyper_L", 0xffed),
    ("Hyper_R", 0xffee),
    ("I", 0x0049),
    ("IO", 0x100000ee),
    ("ISO_Center_Object", 0xfe33),
    ("ISO_Continuous_Underline", 0xfe30),
    ("ISO_Discontinuous_Underline", 0xfe31),
    ("ISO_Emphasize", 0xfe32),
    ("ISO_Enter", 0xfe34),
    ("ISO_Fast_Cursor_Down", 0xfe2f),
    ("ISO_Fast_Cursor_Left", 0xfe2c),
    ("ISO_Fast_Cursor_Right", 0xfe2d),
    ("ISO_Fast_Cursor_Up", 0xfe2e),
    ("ISO_First_Group", 0xfe0c),
    ("ISO_First_Group_Lock", 0xfe0d),
    ("ISO_Group_Latch", 0xfe06),
    ("ISO_Group_Lock", 0xfe07),
    ("ISO_Group_Shift", 0xff7e),
    ("ISO_Last_Group", 0xfe0e),
    ("ISO_Last_Group_Lock", 0xfe0f),
    ("ISO_Left_Tab", 0xfe20),
    ("ISO_Level2_Latch", 0xfe02),
    ("ISO_Level3_Latch", 0xfe04),
    ("ISO_Level3_Lock", 0xfe05),
    ("ISO_Level3_Shift", 0xfe03),
    ("ISO_Level5_Latch", 0xfe12),
    ("ISO_Level5_Lock", 0xfe13),
    ("ISO_Level5_Shift", 0xfe11),
    ("ISO_Lock", 0xfe01),
    ("ISO_Move_Line_Down", 0xfe22),
    ("ISO_Move_Line_Up", 0xfe21),
    ("ISO_Next_Group", 0xfe08),
    ("ISO_Next_Group_Lock", 0xfe09),
    ("ISO_Partial_Line_Down", 0xfe24),
    ("ISO_Partial_Line_Up", 0xfe23),
    ("ISO_Partial_Space_Left", 0xfe25),
    ("ISO_Partial_Space_Right", 0xfe26),
    ("ISO_Prev_Group", 0xfe0a),
    ("ISO_Prev_Group_Lock", 0xfe0b),
    ("ISO_Release_Both_Margins", 0xfe2b),
    ("ISO_Release_Margin_Left", 0xfe29),
    ("ISO_Release_Margin_Right", 0xfe2a),
    ("ISO_Set_Margin_Left", 0xfe27),
    ("ISO_Set_Margin_Right", 0xfe28),
    ("Iabovedot", 0x02a9),
    ("Iacute", 0x00cd),
    ("Ibelowdot", 0x1001eca),
    ("Ibreve", 0x100012c),
    ("Icircumflex", 0x00ce),
    ("Idiaeresis", 0x00cf),
    ("Igrave", 0x00cc),
    ("Ihook", 0x1001ec8),
    ("Imacron", 0x03cf),
    ("Insert", 0xff63),
    ("InsertChar", 0x1000ff72),
    ("InsertLine", 0x1000ff70),
    ("Iogonek", 0x03c7),
    ("Itilde", 0x03a5),
    ("J", 0x004a),
    ("Jcircumflex", 0x02ac),
    ("K", 0x004b),
    ("KP_0", 0xffb0),
    ("KP_1", 0xffb1),
    ("KP_2", 0xffb2),
    ("KP_3", 0xffb3),
    ("KP_4", 0xffb4),
    ("KP_5", 0xffb5),
    ("KP_6", 0xffb6),
    ("KP_7", 0xffb7),
    ("KP_8", 0xffb8),
    ("KP_9", 0xffb9),
    ("KP_Add", 0xffab),
    ("KP_BackTab", 0x1000ff75),
    ("KP_Begin", 0xff9d),
    ("KP_Decimal", 0xffae),
    ("KP_Delete", 0xff9f),
    ("KP_Divide", 0xffaf),
    ("KP_Down", 0xff99),
    ("KP_End", 0xff9c),
    ("KP_Enter", 0xff8d),
    ("KP_Equal", 0xffbd),
    ("KP_F1", 0xff91),
    ("KP_F2", 0xff92),
    ("KP_F3", 0xff93),
    ("KP_F4", 0xff94),
    ("KP_Home", 0xff95),
    ("KP_Insert", 0xff9e),
    ("KP_Left", 0xff96),
    ("KP_Multiply", 0xffaa),
    ("KP_Next", 0xff9b),
    ("KP_Page_Down", 0xff9b),
    ("KP_Page_Up", 0xff9a),
    ("KP_Prior", 0xff9a),
    ("KP_Right", 0xff98),
    ("KP_Separator", 0xffac),
    ("KP_Space", 0xff80),
    ("KP_Subtract", 0xffad),
    ("KP_Tab", 0xff89),
    ("KP_Up", 0xff97),
    ("Kana_Lock", 0xff2d),
    ("Kana_Shift", 0xff2e),
    ("Kanji", 0xff21),
    ("Kanji_Bangou", 0xff37),
    ("Katakana", 0xff26),
    ("Kcedilla", 0x03d3),
    ("Korean_Won", 0x0eff),
    ("L", 0x004c),
    ("L1", 0xffc8),
    ("L10", 0xffd1),
    ("L2", 0xffc9),
    ("L3", 0xffca),
    ("L4", 0xffcb),
    ("L5", 0xffcc),
    ("L6", 0xffcd),
    ("L7", 0xffce),
    ("L8", 0xffcf),
    ("L9", 0xffd0),
    ("Lacute", 0x01c5),
    ("Last_Virtual_Screen", 0xfed4),
    ("Lbelowdot", 0x1001e36),
    ("Lcaron", 0x01a5),
    ("Lcedilla", 0x03a6),
    ("Left", 0xff51),
    ("Linefeed", 0xff0a),
    ("LiraSign", 0x10020a4),
    ("Lstroke", 0x01a3),
    ("M", 0x004d),
    ("Mabovedot", 0x1001e40),
    ("Macedonia_DSE", 0x06b5),
    ("Macedonia_GJE", 0x06b2),
    ("Macedonia_KJE", 0x06bc),
    ("Macedonia_dse", 0x06a5),
    ("Macedonia_gje", 0x06a2),
    ("Macedonia_kje", 0x06ac),
    ("Mae_Koho", 0xff3e),
    ("Massyo", 0xff2c),
    ("Menu", 0xff67),
    ("Meta_L", 0xffe7),
    ("Meta_R", 0xffe8),
    ("MillSign", 0x10020a5),
    ("Mode_switch", 0xff7e),
    ("MouseKeys_Accel_Enable", 0xfe77),
    ("MouseKeys_Enable", 0xfe76),
    ("Muhenkan", 0xff22),
    ("Multi_key", 0xff20),
    ("MultipleCandidate", 0xff3d),
    ("N", 0x004e),
    ("Nacute", 0x01d1),
    ("NairaSign", 0x10020a6),
    ("Ncaron", 0x01d2),
    ("Ncedilla", 0x03d1),
    ("NewSheqelSign", 0x10020aa),
    ("Next", 0xff56),
    ("Next_Virtual_Screen", 0xfed2),
    ("NoSymbol", 0x000000),
    ("Ntilde", 0x00d1),
    ("Num_Lock", 0xff7f),
    ("O", 0x004f),
    ("OE", 0x13bc),
    ("Oacute", 0x00d3),
    ("Obarred", 0x100019f),
    ("Obelowdot", 0x1001ecc),
    ("Ocaron", 0x10001d1),
    ("Ocircumflex", 0x00d4),
    ("Ocircumflexacute", 0x1001ed0),
    ("Ocircumflexbelowdot", 0x1001ed8),
    ("Ocircumflexgrave", 0x1001ed2),
    ("Ocircumflexhook", 0x1001ed4),
    ("Ocircumflextilde", 0x1001ed6),
    ("Odiaeresis", 0x00d6),
    ("Odoubleacute", 0x01d5),
    ("Ograve", 0x00d2),
    ("Ohook", 0x1001ece),
    ("Ohorn", 0x10001a0),
    ("Ohornacute", 0x1001eda),
    ("Ohornbelowdot", 0x1001ee2),
    ("Ohorngrave", 0x1001edc),
    ("Ohornhook", 0x1001ede),
    ("Ohorntilde", 0x1001ee0),
    ("Omacron", 0x03d2),
    ("Ooblique", 0x00d8),
    ("Oslash", 0x00d8),
    ("Otilde", 0x00d5),
    ("Overlay1_Enable", 0xfe78),
    ("Overlay2_Enable", 0xfe79),
    ("P", 0x0050),
    ("Pabovedot", 0x1001e56),
    ("Page_Down", 0xff56),
    ("Page_Up", 0xff55),
    ("Pause", 0xff13),
    ("PesetaSign", 0x10020a7),
    ("Pointer_Accelerate", 0xfefa),
    ("Pointer_Button1", 0xfee9),
    ("Pointer_Button2", 0xfeea),
    ("Pointer_Button3", 0xfeeb),
    ("Pointer_Button4", 0xfeec),
    ("Pointer_Button5", 0xfeed),
    ("Pointer_Button_Dflt", 0xfee8),
    ("Pointer_DblClick1", 0xfeef),
    ("Pointer_DblClick2", 0xfef0),
    ("Pointer_DblClick3", 0xfef1),
    ("Pointer_DblClick4", 0xfef2),
    ("Pointer_DblClick5", 0xfef3),
    ("Pointer_DblClick_Dflt", 0xfeee),
    ("Pointer_DfltBtnNext", 0xfefb),
    ("Pointer_DfltBtnPrev", 0xfefc),
    ("Pointer_Down", 0xfee3),
    ("Pointer_DownLeft", 0xfee6),
    ("Pointer_DownRight", 0xfee7),
    ("Pointer_Drag1", 0xfef5),
    ("Pointer_Drag2", 0xfef6),
    ("Pointer_Drag3", 0xfef7),
    ("Pointer_Drag4", 0xfef8),
    ("Pointer_Drag5", 0xfefd),
    ("Pointer_Drag_Dflt", 0xfef4),
    ("Pointer_EnableKeys", 0xfef9),
    ("Pointer_Left", 0xfee0),
    ("Pointer_Right", 0xfee1),
    ("Pointer_Up", 0xfee2),
    ("Pointer_UpLeft", 0xfee4),
    ("Pointer_UpRight", 0xfee5),
    ("Prev_Virtual_Screen", 0xfed1),
    ("PreviousCandidate", 0xff3e),
    ("Print", 0xff61),
    ("Prior", 0xff55),
    ("Q", 0x0051),
    ("R", 0x0052),
    ("R1", 0xffd2),
    ("R10", 0xffdb),
    ("R11", 0xffdc),
    ("R12", 0xffdd),
    ("R13", 0xffde),
    ("R14", 0xffdf),
    ("R15", 0xffe0),
    ("R2", 0xffd3),
    ("R3", 0xffd4),
    ("R4", 0xffd5),
    ("R5", 0xffd6),
    ("R6", 0xffd7),
    ("R7", 0xffd8),
    ("R8", 0xffd9),
    ("R9", 0xffda),
    ("Racute", 0x01c0),
    ("Rcaron", 0x01d8),
    ("Rcedilla", 0x03a3),
    ("Redo", 0xff66),
    ("RepeatKeys_Enable", 0xfe72),
    ("Reset", 0x1000ff6c),
    ("Return", 0xff0d),
    ("Right", 0xff53),
    ("Romaji", 0xff24),
    ("RupeeSign", 0x10020a8),
    ("S", 0x0053),
    ("SCHWA", 0x100018f),
    ("SSHARP", 0x1001e9e),
    ("Sabovedot", 0x1001e60),
    ("Sacute", 0x01a6),
    ("Scaron", 0x01a9),
    ("Scedilla", 0x01aa),
    ("Scircumflex", 0x02de),
    ("Scroll_Lock", 0xff14),
    ("Select", 0xff60),
    ("Serbian_DJE", 0x06b1),
    ("Serbian_DZE", 0x06bf),
    ("Serbian_JE", 0x06b8),
    ("Serbian_LJE", 0x06b9),
    ("Serbian_NJE", 0x06ba),
    ("Serbian_TSHE", 0x06bb),
    ("Serbian_dje", 0x06a1),
    ("Serbian_dze", 0x06af),
    ("Serbian_je", 0x06a8),
    ("Serbian_lje", 0x06a9),
    ("Serbian_nje", 0x06aa),
    ("Serbian_tshe", 0x06ab),
    ("Shift_L", 0xffe1),
    ("Shift_Lock", 0xffe6),
    ("Shift_R", 0xffe2),
    ("SingleCandidate", 0xff3c),
    ("Sinh_a", 0x1000d85),
    ("Sinh_aa", 0x1000d86),
    ("Sinh_aa2", 0x1000dcf),
    ("Sinh_ae", 0x1000d87),
    ("Sinh_ae2", 0x1000dd0),
    ("Sinh_aee", 0x1000d88),
    ("Sinh_aee2", 0x1000dd1),
    ("Sinh_ai", 0x1000d93),
    ("Sinh_ai2", 0x1000ddb),
    ("Sinh_al", 0x1000dca),
    ("Sinh_au", 0x1000d96),
    ("Sinh_au2", 0x1000dde),
    ("Sinh_ba", 0x1000db6),
    ("Sinh_bha", 0x1000db7),
    ("Sinh_ca", 0x1000da0),
    ("Sinh_cha", 0x1000da1),
    ("Sinh_dda", 0x1000da9),
    ("Sinh_ddha", 0x1000daa),
    ("Sinh_dha", 0x1000daf),
    ("Sinh_dhha", 0x1000db0),
    ("Sinh_e", 0x1000d91),
    ("Sinh_e2", 0x1000dd9),
    ("Sinh_ee", 0x1000d92),
    ("Sinh_ee2", 0x1000dda),
    ("Sinh_fa", 0x1000dc6),
    ("Sinh_ga", 0x1000d9c),
    ("Sinh_gha", 0x1000d9d),
    ("Sinh_h2", 0x1000d83),
    ("Sinh_ha", 0x1000dc4),
    ("Sinh_i", 0x1000d89),
    ("Sinh_i2", 0x1000dd2),
    ("Sinh_ii", 0x1000d8a),
    ("Sinh_ii2", 0x1000dd3),
    ("Sinh_ja", 0x1000da2),
    ("Sinh_jha", 0x1000da3),
    ("Sinh_jnya", 0x1000da5),
    ("Sinh_ka", 0x1000d9a),
    ("Sinh_kha", 0x1000d9b),
    ("Sinh_kunddaliya", 0x1000df4),
    ("Sinh_la", 0x1000dbd),
    ("Sinh_lla", 0x1000dc5),
    ("Sinh_lu", 0x1000d8f),
    ("Sinh_lu2", 0x1000ddf),
    ("Sinh_luu", 0x1000d90),
    ("Sinh_luu2", 0x1000df3),
    ("Sinh_ma", 0x1000db8),
    ("Sinh_mba", 0x1000db9),
    ("Sinh_na", 0x1000db1),
    ("Sinh_ndda", 0x1000dac),
    ("Sinh_ndha", 0x1000db3),
    ("Sinh_ng", 0x1000d82),
    ("Sinh_ng2", 0x1000d9e),
    ("Sinh_nga", 0x1000d9f),
    ("Sinh_nja", 0x1000da6),
    ("Sinh_nna", 0x1000dab),
    ("Sinh_nya", 0x1000da4),
    ("Sinh_o", 0x1000d94),
    ("Sinh_o2", 0x1000ddc),
    ("Sinh_oo", 0x1000d95),
    ("Sinh_oo2", 0x1000ddd),
    ("Sinh_pa", 0x1000db4),
    ("Sinh_pha", 0x1000db5),
    ("Sinh_ra", 0x1000dbb),
    ("Sinh_ri", 0x1000d8d),
    ("Sinh_rii", 0x1000d8e),
    ("Sinh_ru2", 0x1000dd8),
    ("Sinh_ruu2", 0x1000df2),
    ("Sinh_sa", 0x1000dc3),
    ("Sinh_sha", 0x1000dc1),
    ("Sinh_ssha", 0x1000dc2),
    ("Sinh_tha", 0x1000dad),
    ("Sinh_thha", 0x1000dae),
    ("Sinh_tta", 0x1000da7),
    ("Sinh_ttha", 0x1000da8),
    ("Sinh_u", 0x1000d8b),
    ("Sinh_u2", 0x1000dd4),
    ("Sinh_uu", 0x1000d8c),
    ("Sinh_uu2", 0x1000dd6),
    ("Sinh_va", 0x1000dc0),
    ("Sinh_ya", 0x1000dba),
    ("SlowKeys_Enable", 0xfe73),
    ("StickyKeys_Enable", 0xfe75),
    ("SunAgain", 0x0000ff66),
    ("SunAltGraph", 0x0000ff7e),
    ("SunAudioLowerVolume", 0x1005ff77),
    ("SunAudioMute", 0x1005ff78),
    ("SunAudioRaiseVolume", 0x1005ff79),
    ("SunCompose", 0x0000ff20),
    ("SunCopy", 0x1005ff72),
    ("SunCut", 0x1005ff75),
    ("SunF36", 0x1005ff10),
    ("SunF37", 0x1005ff11),
    ("SunFA_Acute", 0x1005ff03),
    ("SunFA_Cedilla", 0x1005ff05),
    ("SunFA_Circum", 0x1005ff01),
    ("SunFA_Diaeresis", 0x1005ff04),
    ("SunFA_Grave", 0x1005ff00),
    ("SunFA_Tilde", 0x1005ff02),
    ("SunFind", 0x0000ff68),
    ("SunFront", 0x1005ff71),
    ("SunOpen", 0x1005ff73),
    ("SunPageDown", 0x0000ff56),
    ("SunPageUp", 0x0000ff55),
    ("SunPaste", 0x1005ff74),
    ("SunPowerSwitch", 0x1005ff76),
    ("SunPowerSwitchShift", 0x1005ff7d),
    ("SunPrint_Screen", 0x0000ff61),
    ("SunProps", 0x1005ff70),
    ("SunStop", 0x0000ff69),
    ("SunSys_Req", 0x1005ff60),
    ("SunUndo", 0x0000ff65),
    ("SunVideoDegauss", 0x1005ff7a),
    ("SunVideoLowerBrightness", 0x1005ff7b),
    ("SunVideoRaiseBrightness", 0x1005ff7c),
    ("Super_L", 0xffeb),
    ("Super_R", 0xffec),
    ("Sys_Req", 0xff15),
    ("System", 0x1000ff6d),
    ("T", 0x0054),
    ("THORN", 0x00de),
    ("Tab", 0xff09),
    ("Tabovedot", 0x1001e6a),
    ("Tcaron", 0x01ab),
    ("Tcedilla", 0x01de),
    ("Terminate_Server", 0xfed5),
    ("Thai_baht", 0x0ddf),
    ("Thai_bobaimai", 0x0dba),
    ("Thai_chochan", 0x0da8),
    ("Thai_chochang", 0x0daa),
    ("Thai_choching", 0x0da9),
    ("Thai_chochoe", 0x0dac),
    ("Thai_dochada", 0x0dae),
    ("Thai_dodek", 0x0db4),
    ("Thai_fofa", 0x0dbd),
    ("Thai_fofan", 0x0dbf),
    ("Thai_hohip", 0x0dcb),
    ("Thai_honokhuk", 0x0dce),
    ("Thai_khokhai", 0x0da2),
    ("Thai_khokhon", 0x0da5),
    ("Thai_khokhuat", 0x0da3),
    ("Thai_khokhwai", 0x0da4),
    ("Thai_khorakhang", 0x0da6),
    ("Thai_kokai", 0x0da1),
    ("Thai_lakkhangyao", 0x0de5),
    ("Thai_lekchet", 0x0df7),
    ("Thai_lekha", 0x0df5),
    ("Thai_lekhok", 0x0df6),
    ("Thai_lekkao", 0x0df9),
    ("Thai_leknung", 0x0df1),
    ("Thai_lekpaet", 0x0df8),
    ("Thai_leksam", 0x0df3),
    ("Thai_leksi", 0x0df4),
    ("Thai_leksong", 0x0df2),
    ("Thai_leksun", 0x0df0),
    ("Thai_lochula", 0x0dcc),
    ("Thai_loling", 0x0dc5),
    ("Thai_lu", 0x0dc6),
    ("Thai_maichattawa", 0x0deb),
    ("Thai_maiek", 0x0de8),
    ("Thai_maihanakat", 0x0dd1),
    ("Thai_maihanakat_maitho", 0x0dde),
    ("Thai_maitaikhu", 0x0de7),
    ("Thai_maitho", 0x0de9),
    ("Thai_maitri", 0x0dea),
    ("Thai_maiyamok", 0x0de6),
    ("Thai_moma", 0x0dc1),
    ("Thai_ngongu", 0x0da7),
    ("Thai_nikhahit", 0x0ded),
    ("Thai_nonen", 0x0db3),
    ("Thai_nonu", 0x0db9),
    ("Thai_oang", 0x0dcd),
    ("Thai_paiyannoi", 0x0dcf),
    ("Thai_phinthu", 0x0dda),
    ("Thai_phophan", 0x0dbe),
    ("Thai_phophung", 0x0dbc),
    ("Thai_phosamphao", 0x0dc0),
    ("Thai_popla", 0x0dbb),
    ("Thai_rorua", 0x0dc3),
    ("Thai_ru", 0x0dc4),
    ("Thai_saraa", 0x0dd0),
    ("Thai_saraaa", 0x0dd2),
    ("Thai_saraae", 0x0de1),
    ("Thai_saraaimaimalai", 0x0de4),
    ("Thai_saraaimaimuan", 0x0de3),
    ("Thai_saraam", 0x0dd3),
    ("Thai_sarae", 0x0de0),
    ("Thai_sarai", 0x0dd4),
    ("Thai_saraii", 0x0dd5),
    ("Thai_sarao", 0x0de2),
    ("Thai_sarau", 0x0dd8),
    ("Thai_saraue", 0x0dd6),
    ("Thai_sarauee", 0x0dd7),
    ("Thai_sarauu", 0x0dd9),
    ("Thai_sorusi", 0x0dc9),
    ("Thai_sosala", 0x0dc8),
    ("Thai_soso", 0x0dab),
    ("Thai_sosua", 0x0dca),
    ("Thai_thanthakhat", 0x0dec),
    ("Thai_thonangmontho", 0x0db1),
    ("Thai_thophuthao", 0x0db2),
    ("Thai_thothahan", 0x0db7),
    ("Thai_thothan", 0x0db0),
    ("Thai_thothong", 0x0db8),
    ("Thai_thothung", 0x0db6),
    ("Thai_topatak", 0x0daf),
    ("Thai_totao", 0x0db5),
    ("Thai_wowaen", 0x0dc7),
    ("Thai_yoyak", 0x0dc2),
    ("Thai_yoying", 0x0dad),
    ("Thorn", 0x00de),
    ("Touroku", 0xff2b),
    ("Tslash", 0x03ac),
    ("U", 0x0055),
    ("Uacute", 0x00da),
    ("Ubelowdot", 0x1001ee4),
    ("Ubreve", 0x02dd),
    ("Ucircumflex", 0x00db),
    ("Udiaeresis", 0x00dc),
    ("Udoubleacute", 0x01db),
    ("Ugrave", 0x00d9),
    ("Uhook", 0x1001ee6),
    ("Uhorn", 0x10001af),
    ("Uhornacute", 0x1001ee8),
    ("Uhornbelowdot", 0x1001ef0),
    ("Uhorngrave", 0x1001eea),
    ("Uhornhook", 0x1001eec),
    ("Uhorntilde", 0x1001eee),
    ("Ukrainian_GHE_WITH_UPTURN", 0x06bd),
    ("Ukrainian_I", 0x06b6),
    ("Ukrainian_IE", 0x06b4),
    ("Ukrainian_YI", 0x06b7),
    ("Ukrainian_ghe_with_upturn", 0x06ad),
    ("Ukrainian_i", 0x06a6),
    ("Ukrainian_ie", 0x06a4),
    ("Ukrainian_yi", 0x06a7),
    ("Ukranian_I", 0x06b6),
    ("Ukranian_JE", 0x06b4),
    ("Ukranian_YI", 0x06b7),
    ("Ukranian_i", 0x06a6),
    ("Ukranian_je", 0x06a4),
    ("Ukranian_yi", 0x06a7),
    ("Umacron", 0x03de),
    ("Undo", 0xff65),
    ("Uogonek", 0x03d9),
    ("Up", 0xff52),
    ("Uring", 0x01d9),
    ("User", 0x1000ff6e),
    ("Utilde", 0x03dd),
    ("V", 0x0056),
    ("VoidSymbol", 0xffffff),
    ("W", 0x0057),
    ("Wacute", 0x1001e82),
    ("Wcircumflex", 0x1000174),
    ("Wdiaeresis", 0x1001e84),
    ("Wgrave", 0x1001e80),
    ("WonSign", 0x10020a9),
    ("X", 0x0058),
    ("XF8610ChannelsDown", 0x100811b9),
    ("XF8610ChannelsUp", 0x100811b8),
    ("XF863DMode", 0x1008126f),
    ("XF86ALSToggle", 0x10081230),
    ("XF86Accessibility", 0x1008124e),
    ("XF86ActionOnSelection", 0x10081254),
    ("XF86AddFavorite", 0x1008ff39),
    ("XF86Addressbook", 0x100811ad),
    ("XF86AppSelect", 0x10081244),
    ("XF86ApplicationLeft", 0x1008ff50),
    ("XF86ApplicationRight", 0x1008ff51),
    ("XF86AspectRatio", 0x10081177),
    ("XF86Assistant", 0x10081247),
    ("XF86AttendantOff", 0x1008121c),
    ("XF86AttendantOn", 0x1008121b),
    ("XF86AttendantToggle", 0x1008121d),
    ("XF86Audio", 0x10081188),
    ("XF86AudioBassBoost", 0x100810d1),
    ("XF86AudioChannelMode", 0x10081175),
    ("XF86AudioCycleTrack", 0x1008ff9b),
    ("XF86AudioDesc", 0x1008126e),
    ("XF86AudioForward", 0x1008ff97),
    ("XF86AudioLowerVolume", 0x1008ff11),
    ("XF86AudioMedia", 0x1008ff32),
    ("XF86AudioMicMute", 0x1008ffb2),
    ("XF86AudioMute", 0x1008ff12),
    ("XF86AudioNext", 0x1008ff17),
    ("XF86AudioPause", 0x1008ff31),
    ("XF86AudioPlay", 0x1008ff14),
    ("XF86AudioPreset", 0x1008ffb6),
    ("XF86AudioPrev", 0x1008ff16),
    ("XF86AudioRaiseVolume", 0x1008ff13),
    ("XF86AudioRandomPlay", 0x1008ff99),
    ("XF86AudioRecord", 0x1008ff1c),
    ("XF86AudioRepeat", 0x1008ff98),
    ("XF86AudioRewind", 0x1008ff3e),
    ("XF86AudioStop", 0x1008ff15),
    ("XF86AutopilotEngageToggle", 0x1008127d),
    ("XF86Away", 0x1008ff8d),
    ("XF86Back", 0x1008ff26),
    ("XF86BackForward", 0x1008ff3f),
    ("XF86Battery", 0x1008ff93),
    ("XF86Blue", 0x1008ffa6),
    ("XF86Bluetooth", 0x1008ff94),
    ("XF86Book", 0x1008ff52),
    ("XF86Break", 0x1008119b),
    ("XF86BrightnessAdjust", 0x1008ff3b),
    ("XF86BrightnessAuto", 0x100810f4),
    ("XF86BrightnessMax", 0x10081251),
    ("XF86BrightnessMin", 0x10081250),
    ("XF86Buttonconfig", 0x10081240),
    ("XF86CD", 0x1008ff53),
    ("XF86Calculater", 0x1008ff54),
    ("XF86Calculator", 0x1008ff1d),
    ("XF86Calendar", 0x1008ff20),
    ("XF86CameraAccessDisable", 0x1008124c),
    ("XF86CameraAccessEnable", 0x1008124b),
    ("XF86CameraAccessToggle", 0x1008124d),
    ("XF86CameraDown", 0x10081218),
    ("XF86CameraFocus", 0x10081210),
    ("XF86CameraLeft", 0x10081219),
    ("XF86CameraRight", 0x1008121a),
    ("XF86CameraUp", 0x10081217),
    ("XF86CameraZoomIn", 0x10081215),
    ("XF86CameraZoomOut", 0x10081216),
    ("XF86ChannelDown", 0x10081193),
    ("XF86ChannelUp", 0x10081192),
    ("XF86Clear", 0x1008ff55),
    ("XF86ClearGrab", 0x1008fe21),
    ("XF86ClearvuSonar", 0x10081286),
    ("XF86Close", 0x1008ff56),
    ("XF86Community", 0x1008ff3d),
    ("XF86ContextMenu", 0x100811b6),
    ("XF86ContextualInsert", 0x10081255),
    ("XF86ContextualQuery", 0x10081256),
    ("XF86ContrastAdjust", 0x1008ff22),
    ("XF86ControlPanel", 0x10081243),
    ("XF86Copy", 0x1008ff57),
    ("XF86Cut", 0x1008ff58),
    ("XF86CycleAngle", 0x1008ff9c),
    ("XF86DOS", 0x1008ff5a),
    ("XF86DVD", 0x10081185),
    ("XF86Data", 0x10081277),
    ("XF86Database", 0x100811aa),
    ("XF86Dictate", 0x1008124a),
    ("XF86Display", 0x1008ff59),
    ("XF86DisplayOff", 0x100810f5),
    ("XF86DisplayToggle", 0x100811af),
    ("XF86DoNotDisturb", 0x1008124f),
    ("XF86Documents", 0x1008ff5b),
    ("XF86DualRangeRadar", 0x10081283),
    ("XF86Editor", 0x100811a6),
    ("XF86Eject", 0x1008ff2c),
    ("XF86ElectronicPrivacyScreenOff", 0x10081253),
    ("XF86ElectronicPrivacyScreenOn", 0x10081252),
    ("XF86EmojiPicker", 0x10081249),
    ("XF86Excel", 0x1008ff5c),
    ("XF86Exit", 0x100810ae),
    ("XF86Explorer", 0x1008ff5d),
    ("XF86FastReverse", 0x10081275),
    ("XF86Favorites", 0x1008ff30),
    ("XF86Finance", 0x1008ff3c),
    ("XF86FishingChart", 0x10081281),
    ("XF86Fn", 0x100811d0),
    ("XF86FnRightShift", 0x100811e5),
    ("XF86Fn_1", 0x100811de),
    ("XF86Fn_2", 0x100811df),
    ("XF86Fn_B", 0x100811e4),
    ("XF86Fn_D", 0x100811e0),
    ("XF86Fn_E", 0x100811e1),
    ("XF86Fn_Esc", 0x100811d1),
    ("XF86Fn_F", 0x100811e2),
    ("XF86Fn_F1", 0x100811d2),
    ("XF86Fn_F10", 0x100811db),
    ("XF86Fn_F11", 0x100811dc),
    ("XF86Fn_F12", 0x100811dd),
    ("XF86Fn_F2", 0x100811d3),
    ("XF86Fn_F3", 0x100811d4),
    ("XF86Fn_F4", 0x100811d5),
    ("XF86Fn_F5", 0x100811d6),
    ("XF86Fn_F6", 0x100811d7),
    ("XF86Fn_F7", 0x100811d8),
    ("XF86Fn_F8", 0x100811d9),
    ("XF86Fn_F9", 0x100811da),
    ("XF86Fn_S", 0x100811e3),
    ("XF86Forward", 0x1008ff27),
    ("XF86FrameBack", 0x1008ff9d),
    ("XF86FrameForward", 0x1008ff9e),
    ("XF86FullScreen", 0x1008ffb8),
    ("XF86Game", 0x1008ff5e),
    ("XF86Go", 0x1008ff5f),
    ("XF86GoTo", 0x10081162),
    ("XF86GraphicsEditor", 0x100811a8),
    ("XF86Green", 0x1008ffa4),
    ("XF86HangupPhone", 0x100811be),
    ("XF86Hibernate", 0x1008ffa8),
    ("XF86History", 0x1008ff37),
    ("XF86HomePage", 0x1008ff18),
    ("XF86HotLinks", 0x1008ff3a),
    ("XF86Images", 0x100811ba),
    ("XF86Info", 0x10081166),
    ("XF86Journal", 0x10081242),
    ("XF86KbdBrightnessDown", 0x1008ff06),
    ("XF86KbdBrightnessUp", 0x1008ff05),
    ("XF86KbdInputAssistAccept", 0x10081264),
    ("XF86KbdInputAssistCancel", 0x10081265),
    ("XF86KbdInputAssistNext", 0x10081261),
    ("XF86KbdInputAssistNextgroup", 0x10081263),
    ("XF86KbdInputAssistPrev", 0x10081260),
    ("XF86KbdInputAssistPrevgroup", 0x10081262),
    ("XF86KbdLcdMenu1", 0x100812b8),
    ("XF86KbdLcdMenu2", 0x100812b9),
    ("XF86KbdLcdMenu3", 0x100812ba),
    ("XF86KbdLcdMenu4", 0x100812bb),
    ("XF86KbdLcdMenu5", 0x100812bc),
    ("XF86KbdLightOnOff", 0x1008ff04),
    ("XF86Keyboard", 0x1008ffb3),
    ("XF86Launch0", 0x1008ff40),
    ("XF86Launch1", 0x1008ff41),
    ("XF86Launch2", 0x1008ff42),
    ("XF86Launch3", 0x1008ff43),
    ("XF86Launch4", 0x1008ff44),
    ("XF86Launch5", 0x1008ff45),
    ("XF86Launch6", 0x1008ff46),
    ("XF86Launch7", 0x1008ff47),
    ("XF86Launch8", 0x1008ff48),
    ("XF86Launch9", 0x1008ff49),
    ("XF86LaunchA", 0x1008ff4a),
    ("XF86LaunchB", 0x1008ff4b),
    ("XF86LaunchC", 0x1008ff4c),
    ("XF86LaunchD", 0x1008ff4d),
    ("XF86LaunchE", 0x1008ff4e),
    ("XF86LaunchF", 0x1008ff4f),
    ("XF86LeftDown", 0x10081269),
    ("XF86LeftUp", 0x10081268),
    ("XF86LightBulb", 0x1008ff35),
    ("XF86LightsToggle", 0x1008121e),
    ("XF86LinkPhone", 0x100811bf),
    ("XF86LogGrabInfo", 0x1008fe25),
    ("XF86LogOff", 0x1008ff61),
    ("XF86LogWindowTree", 0x1008fe24),
    ("XF86Macro1", 0x10081290),
    ("XF86Macro10", 0x10081299),
    ("XF86Macro11", 0x1008129a),
    ("XF86Macro12", 0x1008129b),
    ("XF86Macro13", 0x1008129c),
    ("XF86Macro14", 0x1008129d),
    ("XF86Macro15", 0x1008129e),
    ("XF86Macro16", 0x1008129f),
    ("XF86Macro17", 0x100812a0),
    ("XF86Macro18", 0x100812a1),
    ("XF86Macro19", 0x100812a2),
    ("XF86Macro2", 0x10081291),
    ("XF86Macro20", 0x100812a3),
    ("XF86Macro21", 0x100812a4),
    ("XF86Macro22", 0x100812a5),
    ("XF86Macro23", 0x100812a6),
    ("XF86Macro24", 0x100812a7),
    ("XF86Macro25", 0x100812a8),
    ("XF86Macro26", 0x100812a9),
    ("XF86Macro27", 0x100812aa),
    ("XF86Macro28", 0x100812ab),
    ("XF86Macro29", 0x100812ac),
    ("XF86Macro3", 0x10081292),
    ("XF86Macro30", 0x100812ad),
    ("XF86Macro4", 0x10081293),
    ("XF86Macro5", 0x10081294),
    ("XF86Macro6", 0x10081295),
    ("XF86Macro7", 0x10081296),
    ("XF86Macro8", 0x10081297),
    ("XF86Macro9", 0x10081298),
    ("XF86MacroPreset1", 0x100812b3),
    ("XF86MacroPreset2", 0x100812b4),
    ("XF86MacroPreset3", 0x100812b5),
    ("XF86MacroPresetCycle", 0x100812b2),
    ("XF86MacroRecordStart", 0x100812b0),
    ("XF86MacroRecordStop", 0x100812b1),
    ("XF86Mail", 0x1008ff19),
    ("XF86MailForward", 0x1008ff90),
    ("XF86MarkWaypoint", 0x1008127e),
    ("XF86Market", 0x1008ff62),
    ("XF86MediaLanguageMenu", 0x10081170),
    ("XF86MediaPlayPause", 0x100810a4),
    ("XF86MediaPlaySlow", 0x10081199),
    ("XF86MediaPlayer", 0x10081183),
    ("XF86MediaRepeat", 0x100811b7),
    ("XF86MediaSelectAuxiliary", 0x10081186),
    ("XF86MediaSelectCD", 0x1008ff53),
    ("XF86MediaSelectCable", 0x1008117a),
    ("XF86MediaSelectDVD", 0x10081185),
    ("XF86MediaSelectHome", 0x1008116e),
    ("XF86MediaSelectPC", 0x10081178),
    ("XF86MediaSelectProgramGuide", 0x1008116a),
    ("XF86MediaSelectRadio", 0x10081181),
    ("XF86MediaSelectSatellite", 0x1008117d),
    ("XF86MediaSelectTV", 0x10081179),
    ("XF86MediaSelectTape", 0x10081180),
    ("XF86MediaSelectTeletext", 0x10081184),
    ("XF86MediaSelectTuner", 0x10081182),
    ("XF86MediaSelectVCR", 0x1008117b),
    ("XF86MediaSelectVCRPlus", 0x1008117c),
    ("XF86MediaTitleMenu", 0x10081171),
    ("XF86MediaTopMenu", 0x1008126b),
    ("XF86Meeting", 0x1008ff63),
    ("XF86Memo", 0x1008ff1e),
    ("XF86MenuKB", 0x1008ff65),
    ("XF86MenuPB", 0x1008ff66),
    ("XF86Messenger", 0x1008ff8e),
    ("XF86ModeLock", 0x1008ff01),
    ("XF86MonBrightnessAuto", 0x100810f4),
    ("XF86MonBrightnessCycle", 0x1008ff07),
    ("XF86MonBrightnessDown", 0x1008ff03),
    ("XF86MonBrightnessUp", 0x1008ff02),
    ("XF86Music", 0x1008ff92),
    ("XF86MyComputer", 0x1008ff33),
    ("XF86MySites", 0x1008ff67),
    ("XF86NavChart", 0x10081280),
    ("XF86NavInfo", 0x10081288),
    ("XF86New", 0x1008ff68),
    ("XF86News", 0x1008ff69),
    ("XF86NextElement", 0x1008127b),
    ("XF86NextFavorite", 0x10081270),
    ("XF86Next_VMode", 0x1008fe22),
    ("XF86NotificationCenter", 0x100811bc),
    ("XF86NumberEntryMode", 0x1008119d),
    ("XF86Numeric0", 0x10081200),
    ("XF86Numeric1", 0x10081201),
    ("XF86Numeric11", 0x1008126c),
    ("XF86Numeric12", 0x1008126d),
    ("XF86Numeric2", 0x10081202),
    ("XF86Numeric3", 0x10081203),
    ("XF86Numeric4", 0x10081204),
    ("XF86Numeric5", 0x10081205),
    ("XF86Numeric6", 0x10081206),
    ("XF86Numeric7", 0x10081207),
    ("XF86Numeric8", 0x10081208),
    ("XF86Numeric9", 0x10081209),
    ("XF86NumericA", 0x1008120c),
    ("XF86NumericB", 0x1008120d),
    ("XF86NumericC", 0x1008120e),
    ("XF86NumericD", 0x1008120f),
    ("XF86NumericPound", 0x1008120b),
    ("XF86NumericStar", 0x1008120a),
    ("XF86OK", 0x10081160),
    ("XF86OfficeHome", 0x1008ff6a),
    ("XF86OnScreenKeyboard", 0x10081278),
    ("XF86Open", 0x1008ff6b),
    ("XF86OpenURL", 0x1008ff38),
    ("XF86Option", 0x1008ff6c),
    ("XF86Paste", 0x1008ff6d),
    ("XF86PauseRecord", 0x10081272),
    ("XF86PerformanceMode", 0x100812bd),
    ("XF86Phone", 0x1008ff6e),
    ("XF86PickupPhone", 0x100811bd),
    ("XF86Pictures", 0x1008ff91),
    ("XF86PowerDown", 0x1008ff21),
    ("XF86PowerOff", 0x1008ff2a),
    ("XF86Presentation", 0x100811a9),
    ("XF86Prev_VMode", 0x1008fe23),
    ("XF86PreviousElement", 0x1008127c),
    ("XF86PrivacyScreenToggle", 0x10081279),
    ("XF86Q", 0x1008ff70),
    ("XF86RFKill", 0x1008ffb5),
    ("XF86RadarOverlay", 0x10081284),
    ("XF86Red", 0x1008ffa3),
    ("XF86Refresh", 0x1008ff29),
    ("XF86RefreshRateToggle", 0x10081232),
    ("XF86Reload", 0x1008ff73),
    ("XF86Reply", 0x1008ff72),
    ("XF86RightDown", 0x10081267),
    ("XF86RightUp", 0x10081266),
    ("XF86RockerDown", 0x1008ff24),
    ("XF86RockerEnter", 0x1008ff25),
    ("XF86RockerUp", 0x1008ff23),
    ("XF86RootMenu", 0x1008126a),
    ("XF86RotateWindows", 0x1008ff74),
    ("XF86RotationKB", 0x1008ff76),
    ("XF86RotationLockToggle", 0x1008ffb7),
    ("XF86RotationPB", 0x1008ff75),
    ("XF86Save", 0x1008ff77),
    ("XF86ScreenSaver", 0x1008ff2d),
    ("XF86Screensaver", 0x10081245),
    ("XF86ScrollClick", 0x1008ff7a),
    ("XF86ScrollDown", 0x1008ff79),
    ("XF86ScrollUp", 0x1008ff78),
    ("XF86Search", 0x1008ff1b),
    ("XF86Select", 0x1008ffa0),
    ("XF86SelectiveScreenshot", 0x1008127a),
    ("XF86Send", 0x1008ff7b),
    ("XF86Shop", 0x1008ff36),
    ("XF86SidevuSonar", 0x10081287),
    ("XF86SingleRangeRadar", 0x10081282),
    ("XF86Sleep", 0x1008ff2f),
    ("XF86SlowReverse", 0x10081276),
    ("XF86Sos", 0x1008127f),
    ("XF86Spell", 0x1008ff7c),
    ("XF86SpellCheck", 0x100811b0),
    ("XF86SplitScreen", 0x1008ff7d),
    ("XF86Sport", 0x100810dc),
    ("XF86Standby", 0x1008ff10),
    ("XF86Start", 0x1008ff1a),
    ("XF86Stop", 0x1008ff28),
    ("XF86StopRecord", 0x10081271),
    ("XF86Subtitle", 0x1008ff9a),
    ("XF86Support", 0x1008ff7e),
    ("XF86Suspend", 0x1008ffa7),
    ("XF86Switch_VT_1", 0x1008fe01),
    ("XF86Switch_VT_10", 0x1008fe0a),
    ("XF86Switch_VT_11", 0x1008fe0b),
    ("XF86Switch_VT_12", 0x1008fe0c),
    ("XF86Switch_VT_2", 0x1008fe02),
    ("XF86Switch_VT_3", 0x1008fe03),
    ("XF86Switch_VT_4", 0x1008fe04),
    ("XF86Switch_VT_5", 0x1008fe05),
    ("XF86Switch_VT_6", 0x1008fe06),
    ("XF86Switch_VT_7", 0x1008fe07),
    ("XF86Switch_VT_8", 0x1008fe08),
    ("XF86Switch_VT_9", 0x1008fe09),
    ("XF86TaskPane", 0x1008ff7f),
    ("XF86Taskmanager", 0x10081241),
    ("XF86Terminal", 0x1008ff80),
    ("XF86Time", 0x1008ff9f),
    ("XF86ToDoList", 0x1008ff1f),
    ("XF86Tools", 0x1008ff81),
    ("XF86TopMenu", 0x1008ffa2),
    ("XF86TouchpadOff", 0x1008ffb1),
    ("XF86TouchpadOn", 0x1008ffb0),
    ("XF86TouchpadToggle", 0x1008ffa9),
    ("XF86TraditionalSonar", 0x10081285),
    ("XF86Travel", 0x1008ff82),
    ("XF86UWB", 0x1008ff96),
    ("XF86Ungrab", 0x1008fe20),
    ("XF86Unmute", 0x10081274),
    ("XF86User1KB", 0x1008ff85),
    ("XF86User2KB", 0x1008ff86),
    ("XF86UserPB", 0x1008ff84),
    ("XF86VOD", 0x10081273),
    ("XF86VendorHome", 0x1008ff34),
    ("XF86VendorLogo", 0x10081168),
    ("XF86Video", 0x1008ff87),
    ("XF86VideoPhone", 0x100811a0),
    ("XF86View", 0x1008ffa1),
    ("XF86VoiceCommand", 0x10081246),
    ("XF86Voicemail", 0x100811ac),
    ("XF86WLAN", 0x1008ff95),
    ("XF86WPSButton", 0x10081211),
    ("XF86WWAN", 0x1008ffb4),
    ("XF86WWW", 0x1008ff2e),
    ("XF86WakeUp", 0x1008ff2b),
    ("XF86WebCam", 0x1008ff8f),
    ("XF86WheelButton", 0x1008ff88),
    ("XF86Word", 0x1008ff89),
    ("XF86Xfer", 0x1008ff8a),
    ("XF86Yellow", 0x1008ffa5),
    ("XF86ZoomIn", 0x1008ff8b),
    ("XF86ZoomOut", 0x1008ff8c),
    ("XF86ZoomReset", 0x100811a4),
    ("XF86iTouch", 0x1008ff60),
    ("Xabovedot", 0x1001e8a),
    ("Y", 0x0059),
    ("Yacute", 0x00dd),
    ("Ybelowdot", 0x1001ef4),
    ("Ycircumflex", 0x1000176),
    ("Ydiaeresis", 0x13be),
    ("Ygrave", 0x1001ef2),
    ("Yhook", 0x1001ef6),
    ("Ytilde", 0x1001ef8),
    ("Z", 0x005a),
    ("Zabovedot", 0x01af),
    ("Zacute", 0x01ac),
    ("Zcaron", 0x01ae),
    ("Zen_Koho", 0xff3d),
    ("Zenkaku", 0xff28),
    ("Zenkaku_Hankaku", 0xff2a),
    ("Zstroke", 0x10001b5),
    ("a", 0x0061),
    ("aacute", 0x00e1),
    ("abelowdot", 0x1001ea1),
    ("abovedot", 0x01ff),
    ("abreve", 0x01e3),
    ("abreveacute", 0x1001eaf),
    ("abrevebelowdot", 0x1001eb7),
    ("abrevegrave", 0x1001eb1),
    ("abrevehook", 0x1001eb3),
    ("abrevetilde", 0x1001eb5),
    ("acircumflex", 0x00e2),
    ("acircumflexacute", 0x1001ea5),
    ("acircumflexbelowdot", 0x1001ead),
    ("acircumflexgrave", 0x1001ea7),
    ("acircumflexhook", 0x1001ea9),
    ("acircumflextilde", 0x1001eab),
    ("acute", 0x00b4),
    ("adiaeresis", 0x00e4),
    ("ae", 0x00e6),
    ("agrave", 0x00e0),
    ("ahook", 0x1001ea3),
    ("amacron", 0x03e0),
    ("ampersand", 0x0026),
    ("aogonek", 0x01b1),
    ("apostrophe", 0x0027),
    ("approxeq", 0x1002248),
    ("approximate", 0x08c8),
    ("aring", 0x00e5),
    ("asciicircum", 0x005e),
    ("asciitilde", 0x007e),
    ("asterisk", 0x002a),
    ("at", 0x0040),
    ("atilde", 0x00e3),
    ("b", 0x0062),
    ("babovedot", 0x1001e03),
    ("backslash", 0x005c),
    ("ballotcross", 0x0af4),
    ("bar", 0x007c),
    ("because", 0x1002235),
    ("blank", 0x09df),
    ("block", 0x100000fc),
    ("botintegral", 0x08a5),
    ("botleftparens", 0x08ac),
    ("botleftsqbracket", 0x08a8),
    ("botleftsummation", 0x08b2),
    ("botrightparens", 0x08ae),
    ("botrightsqbracket", 0x08aa),
    ("botrightsummation", 0x08b6),
    ("bott", 0x09f6),
    ("botvertsummationconnector", 0x08b4),
    ("braceleft", 0x007b),
    ("braceright", 0x007d),
    ("bracketleft", 0x005b),
    ("bracketright", 0x005d),
    ("braille_blank", 0x1002800),
    ("braille_dot_1", 0xfff1),
    ("braille_dot_10", 0xfffa),
    ("braille_dot_2", 0xfff2),
    ("braille_dot_3", 0xfff3),
    ("braille_dot_4", 0xfff4),
    ("braille_dot_5", 0xfff5),
    ("braille_dot_6", 0xfff6),
    ("braille_dot_7", 0xfff7),
    ("braille_dot_8", 0xfff8),
    ("braille_dot_9", 0xfff9),
    ("braille_dots_1", 0x1002801),
    ("braille_dots_12", 0x1002803),
    ("braille_dots_123", 0x1002807),
    ("braille_dots_1234", 0x100280f),
    ("braille_dots_12345", 0x100281f),
    ("braille_dots_123456", 0x100283f),
    ("braille_dots_1234567", 0x100287f),
    ("braille_dots_12345678", 0x10028ff),
    ("braille_dots_1234568", 0x10028bf),
    ("braille_dots_123457", 0x100285f),
    ("braille_dots_1234578", 0x10028df),
    ("braille_dots_123458", 0x100289f),
    ("braille_dots_12346", 0x100282f),
    ("braille_dots_123467", 0x100286f),
    ("braille_dots_1234678", 0x10028ef),
    ("braille_dots_123468", 0x10028af),
    ("braille_dots_12347", 0x100284f),
    ("braille_dots_123478", 0x10028cf),
    ("braille_dots_12348", 0x100288f),
    ("braille_dots_1235", 0x1002817),
    ("braille_dots_12356", 0x1002837),
    ("braille_dots_123567", 0x1002877),
    ("braille_dots_1235678", 0x10028f7),
    ("braille_dots_123568", 0x10028b7),
    ("braille_dots_12357", 0x1002857),
    ("braille_dots_123578", 0x10028d7),
    ("braille_dots_12358", 0x1002897),
    ("braille_dots_1236", 0x1002827),
    ("braille_dots_12367", 0x1002867),
    ("braille_dots_123678", 0x10028e7),
    ("braille_dots_12368", 0x10028a7),
    ("braille_dots_1237", 0x1002847),
    ("braille_dots_12378", 0x10028c7),
    ("braille_dots_1238", 0x1002887),
    ("braille_dots_124", 0x100280b),
    ("braille_dots_1245", 0x100281b),
    ("braille_dots_12456", 0x100283b),
    ("braille_dots_124567", 0x100287b),
    ("braille_dots_1245678", 0x10028fb),
    ("braille_dots_124568", 0x10028bb),
    ("braille_dots_12457", 0x100285b),
    ("braille_dots_124578", 0x10028db),
    ("braille_dots_12458", 0x100289b),
    ("braille_dots_1246", 0x100282b),
    ("braille_dots_12467", 0x100286b),
    ("braille_dots_124678", 0x10028eb),
    ("braille_dots_12468", 0x10028ab),
    ("braille_dots_1247", 0x100284b),
    ("braille_dots_12478", 0x10028cb),
    ("braille_dots_1248", 0x100288b),
    ("braille_dots_125", 0x1002813),
    ("braille_dots_1256", 0x1002833),
    ("braille_dots_12567", 0x1002873),
    ("braille_dots_125678", 0x10028f3),
    ("braille_dots_12568", 0x10028b3),
    ("braille_dots_1257", 0x1002853),
    ("braille_dots_12578", 0x10028d3),
    ("braille_dots_1258", 0x1002893),
    ("braille_dots_126", 0x1002823),
    ("braille_dots_1267", 0x1002863),
    ("braille_dots_12678", 0x10028e3),
    ("braille_dots_1268", 0x10028a3),
    ("braille_dots_127", 0x1002843),
    ("braille_dots_1278", 0x10028c3),
    ("braille_dots_128", 0x1002883),
    ("braille_dots_13", 0x1002805),
    ("braille_dots_134", 0x100280d),
    ("braille_dots_1345", 0x100281d),
    ("braille_dots_13456", 0x100283d),
    ("braille_dots_134567", 0x100287d),
    ("braille_dots_1345678", 0x10028fd),
    ("braille_dots_134568", 0x10028bd),
    ("braille_dots_13457", 0x100285d),
    ("braille_dots_134578", 0x10028dd),
    ("braille_dots_13458", 0x100289d),
    ("braille_dots_1346", 0x100282d),
    ("braille_dots_13467", 0x100286d),
    ("braille_dots_134678", 0x10028ed),
    ("braille_dots_13468", 0x10028ad),
    ("braille_dots_1347", 0x100284d),
    ("braille_dots_13478", 0x10028cd),
    ("braille_dots_1348", 0x100288d),
    ("braille_dots_135", 0x1002815),
    ("braille_dots_1356", 0x1002835),
    ("braille_dots_13567", 0x1002875),
    ("braille_dots_135678", 0x10028f5),
    ("braille_dots_13568", 0x10028b5),
    ("braille_dots_1357", 0x1002855),
    ("braille_dots_13578", 0x10028d5),
    ("braille_dots_1358", 0x1002895),
    ("braille_dots_136", 0x1002825),
    ("braille_dots_1367", 0x1002865),
    ("braille_dots_13678", 0x10028e5),
    ("braille_dots_1368", 0x10028a5),
    ("braille_dots_137", 0x1002845),
    ("braille_dots_1378", 0x10028c5),
    ("braille_dots_138", 0x1002885),
    ("braille_dots_14", 0x1002809),
    ("braille_dots_145", 0x1002819),
    ("braille_dots_1456", 0x1002839),
    ("braille_dots_14567", 0x1002879),
    ("braille_dots_145678", 0x10028f9),
    ("braille_dots_14568", 0x10028b9),
    ("braille_dots_1457", 0x1002859),
    ("braille_dots_14578", 0x10028d9),
    ("braille_dots_1458", 0x1002899),
    ("braille_dots_146", 0x1002829),
    ("braille_dots_1467", 0x1002869),
    ("braille_dots_14678", 0x10028e9),
    ("braille_dots_1468", 0x10028a9),
    ("braille_dots_147", 0x1002849),
    ("braille_dots_1478", 0x10028c9),
    ("braille_dots_148", 0x1002889),
    ("braille_dots_15", 0x1002811),
    ("braille_dots_156", 0x1002831),
    ("braille_dots_1567", 0x1002871),
    ("braille_dots_15678", 0x10028f1),
    ("braille_dots_1568", 0x10028b1),
    ("braille_dots_157", 0x1002851),
    ("braille_dots_1578", 0x10028d1),
    ("braille_dots_158", 0x1002891),
    ("braille_dots_16", 0x1002821),
    ("braille_dots_167", 0x1002861),
    ("braille_dots_1678", 0x10028e1),
    ("braille_dots_168", 0x10028a1),
    ("braille_dots_17", 0x1002841),
    ("braille_dots_178", 0x10028c1),
    ("braille_dots_18", 0x1002881),
    ("braille_dots_2", 0x1002802),
    ("braille_dots_23", 0x1002806),
    ("braille_dots_234", 0x100280e),
    ("braille_dots_2345", 0x100281e),
    ("braille_dots_23456", 0x100283e),
    ("braille_dots_234567", 0x100287e),
    ("braille_dots_2345678", 0x10028fe),
    ("braille_dots_234568", 0x10028be),
    ("braille_dots_23457", 0x100285e),
    ("braille_dots_234578", 0x10028de),
    ("braille_dots_23458", 0x100289e),
    ("braille_dots_2346", 0x100282e),
    ("braille_dots_23467", 0x100286e),
    ("braille_dots_234678", 0x10028ee),
    ("braille_dots_23468", 0x10028ae),
    ("braille_dots_2347", 0x100284e),
    ("braille_dots_23478", 0x10028ce),
    ("braille_dots_2348", 0x100288e),
    ("braille_dots_235", 0x1002816),
    ("braille_dots_2356", 0x1002836),
    ("braille_dots_23567", 0x1002876),
    ("braille_dots_235678", 0x10028f6),
    ("braille_dots_23568", 0x10028b6),
    ("braille_dots_2357", 0x1002856),
    ("braille_dots_23578", 0x10028d6),
    ("braille_dots_2358", 0x1002896),
    ("braille_dots_236", 0x1002826),
    ("braille_dots_2367", 0x1002866),
    ("braille_dots_23678", 0x10028e6),
    ("braille_dots_2368", 0x10028a6),
    ("braille_dots_237", 0x1002846),
    ("braille_dots_2378", 0x10028c6),
    ("braille_dots_238", 0x1002886),
    ("braille_dots_24", 0x100280a),
    ("braille_dots_245", 0x100281a),
    ("braille_dots_2456", 0x100283a),
    ("braille_dots_24567", 0x100287a),
    ("braille_dots_245678", 0x10028fa),
    ("braille_dots_24568", 0x10028ba),
    ("braille_dots_2457", 0x100285a),
    ("braille_dots_24578", 0x10028da),
    ("braille_dots_2458", 0x100289a),
    ("braille_dots_246", 0x100282a),
    ("braille_dots_2467", 0x100286a),
    ("braille_dots_24678", 0x10028ea),
    ("braille_dots_2468", 0x10028aa),
    ("braille_dots_247", 0x100284a),
    ("braille_dots_2478", 0x10028ca),
    ("braille_dots_248", 0x100288a),
    ("braille_dots_25", 0x1002812),
    ("braille_dots_256", 0x1002832),
    ("braille_dots_2567", 0x1002872),
    ("braille_dots_25678", 0x10028f2),
    ("braille_dots_2568", 0x10028b2),
    ("braille_dots_257", 0x1002852),
    ("braille_dots_2578", 0x10028d2),
    ("braille_dots_258", 0x1002892),
    ("braille_dots_26", 0x1002822),
    ("braille_dots_267", 0x1002862),
    ("braille_dots_2678", 0x10028e2),
    ("braille_dots_268", 0x10028a2),
    ("braille_dots_27", 0x1002842),
    ("braille_dots_278", 0x10028c2),
    ("braille_dots_28", 0x1002882),
    ("braille_dots_3", 0x1002804),
    ("braille_dots_34", 0x100280c),
    ("braille_dots_345", 0x100281c),
    ("braille_dots_3456", 0x100283c),
    ("braille_dots_34567", 0x100287c),
    ("braille_dots_345678", 0x10028fc),
    ("braille_dots_34568", 0x10028bc),
    ("braille_dots_3457", 0x100285c),
    ("braille_dots_34578", 0x10028dc),
    ("braille_dots_3458", 0x100289c),
    ("braille_dots_346", 0x100282c),
    ("braille_dots_3467", 0x100286c),
    ("braille_dots_34678", 0x10028ec),
    ("braille_dots_3468", 0x10028ac),
    ("braille_dots_347", 0x100284c),
    ("braille_dots_3478", 0x10028cc),
    ("braille_dots_348", 0x100288c),
    ("braille_dots_35", 0x1002814),
    ("braille_dots_356", 0x1002834),
    ("braille_dots_3567", 0x1002874),
    ("braille_dots_35678", 0x10028f4),
    ("braille_dots_3568", 0x10028b4),
    ("braille_dots_357", 0x1002854),
    ("braille_dots_3578", 0x10028d4),
    ("braille_dots_358", 0x1002894),
    ("braille_dots_36", 0x1002824),
    ("braille_dots_367", 0x1002864),
    ("braille_dots_3678", 0x10028e4),
    ("braille_dots_368", 0x10028a4),
    ("braille_dots_37", 0x1002844),
    ("braille_dots_378", 0x10028c4),
    ("braille_dots_38", 0x1002884),
    ("braille_dots_4", 0x1002808),
    ("braille_dots_45", 0x1002818),
    ("braille_dots_456", 0x1002838),
    ("braille_dots_4567", 0x1002878),
    ("braille_dots_45678", 0x10028f8),
    ("braille_dots_4568", 0x10028b8),
    ("braille_dots_457", 0x1002858),
    ("braille_dots_4578", 0x10028d8),
    ("braille_dots_458", 0x1002898),
    ("braille_dots_46", 0x1002828),
    ("braille_dots_467", 0x1002868),
    ("braille_dots_4678", 0x10028e8),
    ("braille_dots_468", 0x10028a8),
    ("braille_dots_47", 0x1002848),
    ("braille_dots_478", 0x10028c8),
    ("braille_dots_48", 0x1002888),
    ("braille_dots_5", 0x1002810),
    ("braille_dots_56", 0x1002830),
    ("braille_dots_567", 0x1002870),
    ("braille_dots_5678", 0x10028f0),
    ("braille_dots_568", 0x10028b0),
    ("braille_dots_57", 0x1002850),
    ("braille_dots_578", 0x10028d0),
    ("braille_dots_58", 0x1002890),
    ("braille_dots_6", 0x1002820),
    ("braille_dots_67", 0x1002860),
    ("braille_dots_678", 0x10028e0),
    ("braille_dots_68", 0x10028a0),
    ("braille_dots_7", 0x1002840),
    ("braille_dots_78", 0x10028c0),
    ("braille_dots_8", 0x1002880),
    ("breve", 0x01a2),
    ("brokenbar", 0x00a6),
    ("c", 0x0063),
    ("c_h", 0xfea3),
    ("cabovedot", 0x02e5),
    ("cacute", 0x01e6),
    ("careof", 0x0ab8),
    ("caret", 0x0afc),
    ("caron", 0x01b7),
    ("ccaron", 0x01e8),
    ("ccedilla", 0x00e7),
    ("ccircumflex", 0x02e6),
    ("cedilla", 0x00b8),
    ("cent", 0x00a2),
    ("ch", 0xfea0),
    ("checkerboard", 0x09e1),
    ("checkmark", 0x0af3),
    ("circle", 0x0bcf),
    ("club", 0x0aec),
    ("colon", 0x003a),
    ("combining_acute", 0x1000301),
    ("combining_belowdot", 0x1000323),
    ("combining_grave", 0x1000300),
    ("combining_hook", 0x1000309),
    ("combining_tilde", 0x1000303),
    ("comma", 0x002c),
    ("containsas", 0x100220b),
    ("copyright", 0x00a9),
    ("cr", 0x09e4),
    ("crossinglines", 0x09ee),
    ("cuberoot", 0x100221b),
    ("currency", 0x00a4),
    ("cursor", 0x0aff),
    ("d", 0x0064),
    ("dabovedot", 0x1001e0b),
    ("dagger", 0x0af1),
    ("dcaron", 0x01ef),
    ("dead_A", 0xfe81),
    ("dead_E", 0xfe83),
    ("dead_I", 0xfe85),
    ("dead_O", 0xfe87),
    ("dead_SCHWA", 0xfe8b),
    ("dead_U", 0xfe89),
    ("dead_a", 0xfe80),
    ("dead_abovecomma", 0xfe64),
    ("dead_abovedot", 0xfe56),
    ("dead_abovereversedcomma", 0xfe65),
    ("dead_abovering", 0xfe58),
    ("dead_aboveverticalline", 0xfe91),
    ("dead_acute", 0xfe51),
    ("dead_apostrophe", 0xfe8e),
    ("dead_belowbreve", 0xfe6b),
    ("dead_belowcircumflex", 0xfe69),
    ("dead_belowcomma", 0xfe6e),
    ("dead_belowdiaeresis", 0xfe6c),
    ("dead_belowdot", 0xfe60),
    ("dead_belowmacron", 0xfe68),
    ("dead_belowring", 0xfe67),
    ("dead_belowtilde", 0xfe6a),
    ("dead_belowverticalline", 0xfe92),
    ("dead_breve", 0xfe55),
    ("dead_capital_schwa", 0xfe8b),
    ("dead_caron", 0xfe5a),
    ("dead_cedilla", 0xfe5b),
    ("dead_circumflex", 0xfe52),
    ("dead_currency", 0xfe6f),
    ("dead_dasia", 0xfe65),
    ("dead_diaeresis", 0xfe57),
    ("dead_doubleacute", 0xfe59),
    ("dead_doublegrave", 0xfe66),
    ("dead_e", 0xfe82),
    ("dead_grave", 0xfe50),
    ("dead_greek", 0xfe8c),
    ("dead_hamza", 0xfe8d),
    ("dead_hook", 0xfe61),
    ("dead_horn", 0xfe62),
    ("dead_i", 0xfe84),
    ("dead_invertedbreve", 0xfe6d),
    ("dead_iota", 0xfe5d),
    ("dead_longsolidusoverlay", 0xfe93),
    ("dead_lowline", 0xfe90),
    ("dead_macron", 0xfe54),
    ("dead_o", 0xfe86),
    ("dead_ogonek", 0xfe5c),
    ("dead_perispomeni", 0xfe53),
    ("dead_psili", 0xfe64),
    ("dead_schwa", 0xfe8a),
    ("dead_semivoiced_sound", 0xfe5f),
    ("dead_small_schwa", 0xfe8a),
    ("dead_stroke", 0xfe63),
    ("dead_tilde", 0xfe53),
    ("dead_u", 0xfe88),
    ("dead_voiced_sound", 0xfe5e),
    ("decimalpoint", 0x0abd),
    ("degree", 0x00b0),
    ("diaeresis", 0x00a8),
    ("diamond", 0x0aed),
    ("digitspace", 0x0aa5),
    ("dintegral", 0x100222c),
    ("division", 0x00f7),
    ("dollar", 0x0024),
    ("doubbaselinedot", 0x0aaf),
    ("doubleacute", 0x01bd),
    ("doubledagger", 0x0af2),
    ("doublelowquotemark", 0x0afe),
    ("downarrow", 0x08fe),
    ("downcaret", 0x0ba8),
    ("downshoe", 0x0bd6),
    ("downstile", 0x0bc4),
    ("downtack", 0x0bc2),
    ("dstroke", 0x01f0),
    ("e", 0x0065),
    ("eabovedot", 0x03ec),
    ("eacute", 0x00e9),
    ("ebelowdot", 0x1001eb9),
    ("ecaron", 0x01ec),
    ("ecircumflex", 0x00ea),
    ("ecircumflexacute", 0x1001ebf),
    ("ecircumflexbelowdot", 0x1001ec7),
    ("ecircumflexgrave", 0x1001ec1),
    ("ecircumflexhook", 0x1001ec3),
    ("ecircumflextilde", 0x1001ec5),
    ("ediaeresis", 0x00eb),
    ("egrave", 0x00e8),
    ("ehook", 0x1001ebb),
    ("eightsubscript", 0x1002088),
    ("eightsuperior", 0x1002078),
    ("elementof", 0x1002208),
    ("ellipsis", 0x0aae),
    ("em3space", 0x0aa3),
    ("em4space", 0x0aa4),
    ("emacron", 0x03ba),
    ("emdash", 0x0aa9),
    ("emfilledcircle", 0x0ade),
    ("emfilledrect", 0x0adf),
    ("emopencircle", 0x0ace),
    ("emopenrectangle", 0x0acf),
    ("emptyset", 0x1002205),
    ("emspace", 0x0aa1),
    ("endash", 0x0aaa),
    ("enfilledcircbullet", 0x0ae6),
    ("enfilledsqbullet", 0x0ae7),
    ("eng", 0x03bf),
    ("enopencircbullet", 0x0ae0),
    ("enopensquarebullet", 0x0ae1),
    ("enspace", 0x0aa2),
    ("eogonek", 0x01ea),
    ("equal", 0x003d),
    ("eth", 0x00f0),
    ("etilde", 0x1001ebd),
    ("exclam", 0x0021),
    ("exclamdown", 0x00a1),
    ("ezh", 0x1000292),
    ("f", 0x0066),
    ("fabovedot", 0x1001e1f),
    ("femalesymbol", 0x0af8),
    ("ff", 0x09e3),
    ("figdash", 0x0abb),
    ("filledlefttribullet", 0x0adc),
    ("filledrectbullet", 0x0adb),
    ("filledrighttribullet", 0x0add),
    ("filledtribulletdown", 0x0ae9),
    ("filledtribulletup", 0x0ae8),
    ("fiveeighths", 0x0ac5),
    ("fivesixths", 0x0ab7),
    ("fivesubscript", 0x1002085),
    ("fivesuperior", 0x1002075),
    ("fourfifths", 0x0ab5),
    ("foursubscript", 0x1002084),
    ("foursuperior", 0x1002074),
    ("fourthroot", 0x100221c),
    ("function", 0x08f6),
    ("g", 0x0067),
    ("gabovedot", 0x02f5),
    ("gbreve", 0x02bb),
    ("gcaron", 0x10001e7),
    ("gcedilla", 0x03bb),
    ("gcircumflex", 0x02f8),
    ("grave", 0x0060),
    ("greater", 0x003e),
    ("greaterthanequal", 0x08be),
    ("guilder", 0x100000be),
    ("guillemetleft", 0x00ab),
    ("guillemetright", 0x00bb),
    ("guillemotleft", 0x00ab),
    ("guillemotright", 0x00bb),
    ("h", 0x0068),
    ("hairspace", 0x0aa8),
    ("hcircumflex", 0x02b6),
    ("heart", 0x0aee),
    ("hebrew_aleph", 0x0ce0),
    ("hebrew_ayin", 0x0cf2),
    ("hebrew_bet", 0x0ce1),
    ("hebrew_beth", 0x0ce1),
    ("hebrew_chet", 0x0ce7),
    ("hebrew_dalet", 0x0ce3),
    ("hebrew_daleth", 0x0ce3),
    ("hebrew_doublelowline", 0x0cdf),
    ("hebrew_finalkaph", 0x0cea),
    ("hebrew_finalmem", 0x0ced),
    ("hebrew_finalnun", 0x0cef),
    ("hebrew_finalpe", 0x0cf3),
    ("hebrew_finalzade", 0x0cf5),
    ("hebrew_finalzadi", 0x0cf5),
    ("hebrew_gimel", 0x0ce2),
    ("hebrew_gimmel", 0x0ce2),
    ("hebrew_he", 0x0ce4),
    ("hebrew_het", 0x0ce7),
    ("hebrew_kaph", 0x0ceb),
    ("hebrew_kuf", 0x0cf7),
    ("hebrew_lamed", 0x0cec),
    ("hebrew_mem", 0x0cee),
    ("hebrew_nun", 0x0cf0),
    ("hebrew_pe", 0x0cf4),
    ("hebrew_qoph", 0x0cf7),
    ("hebrew_resh", 0x0cf8),
    ("hebrew_samech", 0x0cf1),
    ("hebrew_samekh", 0x0cf1),
    ("hebrew_shin", 0x0cf9),
    ("hebrew_taf", 0x0cfa),
    ("hebrew_taw", 0x0cfa),
    ("hebrew_tet", 0x0ce8),
    ("hebrew_teth", 0x0ce8),
    ("hebrew_waw", 0x0ce5),
    ("hebrew_yod", 0x0ce9),
    ("hebrew_zade", 0x0cf6),
    ("hebrew_zadi", 0x0cf6),
    ("hebrew_zain", 0x0ce6),
    ("hebrew_zayin", 0x0ce6),
    ("hexagram", 0x0ada),
    ("horizconnector", 0x08a3),
    ("horizlinescan1", 0x09ef),
    ("horizlinescan3", 0x09f0),
    ("horizlinescan5", 0x09f1),
    ("horizlinescan7", 0x09f2),
    ("horizlinescan9", 0x09f3),
    ("hpBackTab", 0x1000ff74),
    ("hpClearLine", 0x1000ff6f),
    ("hpDeleteChar", 0x1000ff73),
    ("hpDeleteLine", 0x1000ff71),
    ("hpIO", 0x100000ee),
    ("hpInsertChar", 0x1000ff72),
    ("hpInsertLine", 0x1000ff70),
    ("hpKP_BackTab", 0x1000ff75),
    ("hpModelock1", 0x1000ff48),
    ("hpModelock2", 0x1000ff49),
    ("hpReset", 0x1000ff6c),
    ("hpSystem", 0x1000ff6d),
    ("hpUser", 0x1000ff6e),
    ("hpYdiaeresis", 0x100000ee),
    ("hpblock", 0x100000fc),
    ("hpguilder", 0x100000be),
    ("hplira", 0x100000af),
    ("hplongminus", 0x100000f6),
    ("hpmute_acute", 0x100000a8),
    ("hpmute_asciicircum", 0x100000aa),
    ("hpmute_asciitilde", 0x100000ac),
    ("hpmute_diaeresis", 0x100000ab),
    ("hpmute_grave", 0x100000a9),
    ("hstroke", 0x02b1),
    ("ht", 0x09e2),
    ("hyphen", 0x00ad),
    ("i", 0x0069),
    ("iacute", 0x00ed),
    ("ibelowdot", 0x1001ecb),
    ("ibreve", 0x100012d),
    ("icircumflex", 0x00ee),
    ("identical", 0x08cf),
    ("idiaeresis", 0x00ef),
    ("idotless", 0x02b9),
    ("ifonlyif", 0x08cd),
    ("igrave", 0x00ec),
    ("ihook", 0x1001ec9),
    ("imacron", 0x03ef),
    ("implies", 0x08ce),
    ("includedin", 0x08da),
    ("includes", 0x08db),
    ("infinity", 0x08c2),
    ("integral", 0x08bf),
    ("intersection", 0x08dc),
    ("iogonek", 0x03e7),
    ("itilde", 0x03b5),
    ("j", 0x006a),
    ("jcircumflex", 0x02bc),
    ("jot", 0x0bca),
    ("k", 0x006b),
    ("kana_A", 0x04b1),
    ("kana_CHI", 0x04c1),
    ("kana_E", 0x04b4),
    ("kana_FU", 0x04cc),
    ("kana_HA", 0x04ca),
    ("kana_HE", 0x04cd),
    ("kana_HI", 0x04cb),
    ("kana_HO", 0x04ce),
    ("kana_HU", 0x04cc),
    ("kana_I", 0x04b2),
    ("kana_KA", 0x04b6),
    ("kana_KE", 0x04b9),
    ("kana_KI", 0x04b7),
    ("kana_KO", 0x04ba),
    ("kana_KU", 0x04b8),
    ("kana_MA", 0x04cf),
    ("kana_ME", 0x04d2),
    ("kana_MI", 0x04d0),
    ("kana_MO", 0x04d3),
    ("kana_MU", 0x04d1),
    ("kana_N", 0x04dd),
    ("kana_NA", 0x04c5),
    ("kana_NE", 0x04c8),
    ("kana_NI", 0x04c6),
    ("kana_NO", 0x04c9),
    ("kana_NU", 0x04c7),
    ("kana_O", 0x04b5),
    ("kana_RA", 0x04d7),
    ("kana_RE", 0x04da),
    ("kana_RI", 0x04d8),
    ("kana_RO", 0x04db),
    ("kana_RU", 0x04d9),
    ("kana_SA", 0x04bb),
    ("kana_SE", 0x04be),
    ("kana_SHI", 0x04bc),
    ("kana_SO", 0x04bf),
    ("kana_SU", 0x04bd),
    ("kana_TA", 0x04c0),
    ("kana_TE", 0x04c3),
    ("kana_TI", 0x04c1),
    ("kana_TO", 0x04c4),
    ("kana_TSU", 0x04c2),
    ("kana_TU", 0x04c2),
    ("kana_U", 0x04b3),
    ("kana_WA", 0x04dc),
    ("kana_WO", 0x04a6),
    ("kana_YA", 0x04d4),
    ("kana_YO", 0x04d6),
    ("kana_YU", 0x04d5),
    ("kana_a", 0x04a7),
    ("kana_closingbracket", 0x04a3),
    ("kana_comma", 0x04a4),
    ("kana_conjunctive", 0x04a5),
    ("kana_e", 0x04aa),
    ("kana_fullstop", 0x04a1),
    ("kana_i", 0x04a8),
    ("kana_middledot", 0x04a5),
    ("kana_o", 0x04ab),
    ("kana_openingbracket", 0x04a2),
    ("kana_switch", 0xff7e),
    ("kana_tsu", 0x04af),
    ("kana_tu", 0x04af),
    ("kana_u", 0x04a9),
    ("kana_ya", 0x04ac),
    ("kana_yo", 0x04ae),
    ("kana_yu", 0x04ad),
    ("kappa", 0x03a2),
    ("kcedilla", 0x03f3),
    ("kra", 0x03a2),
    ("l", 0x006c),
    ("lacute", 0x01e5),
    ("latincross", 0x0ad9),
    ("lbelowdot", 0x1001e37),
    ("lcaron", 0x01b5),
    ("lcedilla", 0x03b6),
    ("leftanglebracket", 0x0abc),
    ("leftarrow", 0x08fb),
    ("leftcaret", 0x0ba3),
    ("leftdoublequotemark", 0x0ad2),
    ("leftmiddlecurlybrace", 0x08af),
    ("leftopentriangle", 0x0acc),
    ("leftpointer", 0x0aea),
    ("leftradical", 0x08a1),
    ("leftshoe", 0x0bda),
    ("leftsingleanglequotemark", 0x1002039),
    ("leftsinglequotemark", 0x0ad0),
    ("leftt", 0x09f4),
    ("lefttack", 0x0bdc),
    ("less", 0x003c),
    ("lessthanequal", 0x08bc),
    ("lf", 0x09e5),
    ("lira", 0x100000af),
    ("logicaland", 0x08de),
    ("logicalor", 0x08df),
    ("longminus", 0x100000f6),
    ("lowleftcorner", 0x09ed),
    ("lowrightcorner", 0x09ea),
    ("lstroke", 0x01b3),
    ("m", 0x006d),
    ("mabovedot", 0x1001e41),
    ("macron", 0x00af),
    ("malesymbol", 0x0af7),
    ("maltesecross", 0x0af0),
    ("marker", 0x0abf),
    ("masculine", 0x00ba),
    ("minus", 0x002d),
    ("minutes", 0x0ad6),
    ("mu", 0x00b5),
    ("multiply", 0x00d7),
    ("musicalflat", 0x0af6),
    ("musicalsharp", 0x0af5),
    ("mute_acute", 0x100000a8),
    ("mute_asciicircum", 0x100000aa),
    ("mute_asciitilde", 0x100000ac),
    ("mute_diaeresis", 0x100000ab),
    ("mute_grave", 0x100000a9),
    ("n", 0x006e),
    ("nabla", 0x08c5),
    ("nacute", 0x01f1),
    ("ncaron", 0x01f2),
    ("ncedilla", 0x03f1),
    ("ninesubscript", 0x1002089),
    ("ninesuperior", 0x1002079),
    ("nl", 0x09e8),
    ("nobreakspace", 0x00a0),
    ("notapproxeq", 0x1002247),
    ("notelementof", 0x1002209),
    ("notequal", 0x08bd),
    ("notidentical", 0x1002262),
    ("notsign", 0x00ac),
    ("ntilde", 0x00f1),
    ("numbersign", 0x0023),
    ("numerosign", 0x06b0),
    ("o", 0x006f),
    ("oacute", 0x00f3),
    ("obarred", 0x1000275),
    ("obelowdot", 0x1001ecd),
    ("ocaron", 0x10001d2),
    ("ocircumflex", 0x00f4),
    ("ocircumflexacute", 0x1001ed1),
    ("ocircumflexbelowdot", 0x1001ed9),
    ("ocircumflexgrave", 0x1001ed3),
    ("ocircumflexhook", 0x1001ed5),
    ("ocircumflextilde", 0x1001ed7),
    ("odiaeresis", 0x00f6),
    ("odoubleacute", 0x01f5),
    ("oe", 0x13bd),
    ("ogonek", 0x01b2),
    ("ograve", 0x00f2),
    ("ohook", 0x1001ecf),
    ("ohorn", 0x10001a1),
    ("ohornacute", 0x1001edb),
    ("ohornbelowdot", 0x1001ee3),
    ("ohorngrave", 0x1001edd),
    ("ohornhook", 0x1001edf),
    ("ohorntilde", 0x1001ee1),
    ("omacron", 0x03f2),
    ("oneeighth", 0x0ac3),
    ("onefifth", 0x0ab2),
    ("onehalf", 0x00bd),
    ("onequarter", 0x00bc),
    ("onesixth", 0x0ab6),
    ("onesubscript", 0x1002081),
    ("onesuperior", 0x00b9),
    ("onethird", 0x0ab0),
    ("ooblique", 0x00f8),
    ("openrectbullet", 0x0ae2),
    ("openstar", 0x0ae5),
    ("opentribulletdown", 0x0ae4),
    ("opentribulletup", 0x0ae3),
    ("ordfeminine", 0x00aa),
    ("ordmasculine", 0x00ba),
    ("osfActivate", 0x1004ff44),
    ("osfAddMode", 0x1004ff31),
    ("osfBackSpace", 0x1004ff08),
    ("osfBackTab", 0x1004ff07),
    ("osfBeginData", 0x1004ff5a),
    ("osfBeginLine", 0x1004ff58),
    ("osfCancel", 0x1004ff69),
    ("osfClear", 0x1004ff0b),
    ("osfCopy", 0x1004ff02),
    ("osfCut", 0x1004ff03),
    ("osfDelete", 0x1004ffff),
    ("osfDeselectAll", 0x1004ff72),
    ("osfDown", 0x1004ff54),
    ("osfEndData", 0x1004ff59),
    ("osfEndLine", 0x1004ff57),
    ("osfEscape", 0x1004ff1b),
    ("osfExtend", 0x1004ff74),
    ("osfHelp", 0x1004ff6a),
    ("osfInsert", 0x1004ff63),
    ("osfLeft", 0x1004ff51),
    ("osfMenu", 0x1004ff67),
    ("osfMenuBar", 0x1004ff45),
    ("osfNextField", 0x1004ff5e),
    ("osfNextMenu", 0x1004ff5c),
    ("osfPageDown", 0x1004ff42),
    ("osfPageLeft", 0x1004ff40),
    ("osfPageRight", 0x1004ff43),
    ("osfPageUp", 0x1004ff41),
    ("osfPaste", 0x1004ff04),
    ("osfPrevField", 0x1004ff5d),
    ("osfPrevMenu", 0x1004ff5b),
    ("osfPrimaryPaste", 0x1004ff32),
    ("osfQuickPaste", 0x1004ff33),
    ("osfReselect", 0x1004ff73),
    ("osfRestore", 0x1004ff78),
    ("osfRight", 0x1004ff53),
    ("osfSelect", 0x1004ff60),
    ("osfSelectAll", 0x1004ff71),
    ("osfUndo", 0x1004ff65),
    ("osfUp", 0x1004ff52),
    ("oslash", 0x00f8),
    ("otilde", 0x00f5),
    ("overbar", 0x0bc0),
    ("overline", 0x047e),
    ("p", 0x0070),
    ("pabovedot", 0x1001e57),
    ("paragraph", 0x00b6),
    ("parenleft", 0x0028),
    ("parenright", 0x0029),
    ("partdifferential", 0x1002202),
    ("partialderivative", 0x08ef),
    ("percent", 0x0025),
    ("period", 0x002e),
    ("periodcentered", 0x00b7),
    ("permille", 0x0ad5),
    ("phonographcopyright", 0x0afb),
    ("plus", 0x002b),
    ("plusminus", 0x00b1),
    ("prescription", 0x0ad4),
    ("prolongedsound", 0x04b0),
    ("punctspace", 0x0aa6),
    ("q", 0x0071),
    ("quad", 0x0bcc),
    ("question", 0x003f),
    ("questiondown", 0x00bf),
    ("quotedbl", 0x0022),
    ("quoteleft", 0x0060),
    ("quoteright", 0x0027),
    ("r", 0x0072),
    ("racute", 0x01e0),
    ("radical", 0x08d6),
    ("rcaron", 0x01f8),
    ("rcedilla", 0x03b3),
    ("registered", 0x00ae),
    ("rightanglebracket", 0x0abe),
    ("rightarrow", 0x08fd),
    ("rightcaret", 0x0ba6),
    ("rightdoublequotemark", 0x0ad3),
    ("rightmiddlecurlybrace", 0x08b0),
    ("rightmiddlesummation", 0x08b7),
    ("rightopentriangle", 0x0acd),
    ("rightpointer", 0x0aeb),
    ("rightshoe", 0x0bd8),
    ("rightsingleanglequotemark", 0x100203a),
    ("rightsinglequotemark", 0x0ad1),
    ("rightt", 0x09f5),
    ("righttack", 0x0bfc),
    ("s", 0x0073),
    ("sabovedot", 0x1001e61),
    ("sacute", 0x01b6),
    ("scaron", 0x01b9),
    ("scedilla", 0x01ba),
    ("schwa", 0x1000259),
    ("scircumflex", 0x02fe),
    ("script_switch", 0xff7e),
    ("seconds", 0x0ad7),
    ("section", 0x00a7),
    ("semicolon", 0x003b),
    ("semivoicedsound", 0x04df),
    ("seveneighths", 0x0ac6),
    ("sevensubscript", 0x1002087),
    ("sevensuperior", 0x1002077),
    ("signaturemark", 0x0aca),
    ("signifblank", 0x0aac),
    ("similarequal", 0x08c9),
    ("singlelowquotemark", 0x0afd),
    ("sixsubscript", 0x1002086),
    ("sixsuperior", 0x1002076),
    ("slash", 0x002f),
    ("soliddiamond", 0x09e0),
    ("space", 0x0020),
    ("squareroot", 0x100221a),
    ("ssharp", 0x00df),
    ("sterling", 0x00a3),
    ("stricteq", 0x1002263),
    ("t", 0x0074),
    ("tabovedot", 0x1001e6b),
    ("tcaron", 0x01bb),
    ("tcedilla", 0x01fe),
    ("telephone", 0x0af9),
    ("telephonerecorder", 0x0afa),
    ("therefore", 0x08c0),
    ("thinspace", 0x0aa7),
    ("thorn", 0x00fe),
    ("threeeighths", 0x0ac4),
    ("threefifths", 0x0ab4),
    ("threequarters", 0x00be),
    ("threesubscript", 0x1002083),
    ("threesuperior", 0x00b3),
    ("tintegral", 0x100222d),
    ("topintegral", 0x08a4),
    ("topleftparens", 0x08ab),
    ("topleftradical", 0x08a2),
    ("topleftsqbracket", 0x08a7),
    ("topleftsummation", 0x08b1),
    ("toprightparens", 0x08ad),
    ("toprightsqbracket", 0x08a9),
    ("toprightsummation", 0x08b5),
    ("topt", 0x09f7),
    ("topvertsummationconnector", 0x08b3),
    ("trademark", 0x0ac9),
    ("trademarkincircle", 0x0acb),
    ("tslash", 0x03bc),
    ("twofifths", 0x0ab3),
    ("twosubscript", 0x1002082),
    ("twosuperior", 0x00b2),
    ("twothirds", 0x0ab1),
    ("u", 0x0075),
    ("uacute", 0x00fa),
    ("ubelowdot", 0x1001ee5),
    ("ubreve", 0x02fd),
    ("ucircumflex", 0x00fb),
    ("udiaeresis", 0x00fc),
    ("udoubleacute", 0x01fb),
    ("ugrave", 0x00f9),
    ("uhook", 0x1001ee7),
    ("uhorn", 0x10001b0),
    ("uhornacute", 0x1001ee9),
    ("uhornbelowdot", 0x1001ef1),
    ("uhorngrave", 0x1001eeb),
    ("uhornhook", 0x1001eed),
    ("uhorntilde", 0x1001eef),
    ("umacron", 0x03fe),
    ("underbar", 0x0bc6),
    ("underscore", 0x005f),
    ("union", 0x08dd),
    ("uogonek", 0x03f9),
    ("uparrow", 0x08fc),
    ("upcaret", 0x0ba9),
    ("upleftcorner", 0x09ec),
    ("uprightcorner", 0x09eb),
    ("upshoe", 0x0bc3),
    ("upstile", 0x0bd3),
    ("uptack", 0x0bce),
    ("uring", 0x01f9),
    ("utilde", 0x03fd),
    ("v", 0x0076),
    ("variation", 0x08c1),
    ("vertbar", 0x09f8),
    ("vertconnector", 0x08a6),
    ("voicedsound", 0x04de),
    ("vt", 0x09e9),
    ("w", 0x0077),
    ("wacute", 0x1001e83),
    ("wcircumflex", 0x1000175),
    ("wdiaeresis", 0x1001e85),
    ("wgrave", 0x1001e81),
    ("x", 0x0078),
    ("xabovedot", 0x1001e8b),
    ("y", 0x0079),
    ("yacute", 0x00fd),
    ("ybelowdot", 0x1001ef5),
    ("ycircumflex", 0x1000177),
    ("ydiaeresis", 0x00ff),
    ("yen", 0x00a5),
    ("ygrave", 0x1001ef3),
    ("yhook", 0x1001ef7),
    ("ytilde", 0x1001ef9),
    ("z", 0x007a),
    ("zabovedot", 0x01bf),
    ("zacute", 0x01bc),
    ("zcaron", 0x01be),
    ("zerosubscript", 0x1002080),
    ("zerosuperior", 0x1002070),
    ("zstroke", 0x10001b6),
];
