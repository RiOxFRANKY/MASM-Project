package com.rioxfranky.dosbox;

final class Cp437 {
    private static final String LOW =
            " ☺☻♥♦♣♠•◘○◙♂♀♪♫☼►◄↕‼¶§▬↨↑↓→←∟↔▲▼";
    private static final String HIGH =
            "ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿⌐¬½¼¡«»"
                    + "░▒▓│┤╡╢╖╕╣║╗╝╜╛┐└┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀"
                    + "αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■ ";

    private static final String[] GLYPHS = new String[256];

    static {
        for (int code = 0; code < 256; code++) {
            char character;
            if (code < 0x20) {
                character = LOW.charAt(code);
            } else if (code == 0x7F) {
                character = '⌂';
            } else if (code >= 0x80) {
                character = HIGH.charAt(code - 0x80);
            } else {
                character = (char) code;
            }
            GLYPHS[code] = String.valueOf(character);
        }
    }

    private Cp437() {
    }

    static String glyph(int code) {
        return GLYPHS[code & 0xFF];
    }
}
