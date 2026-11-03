package com.rioxfranky.dosbox;

final class Palette {
    private static final int[] COLORS = {
            0xFF000000, 0xFF0000AA, 0xFF00AA00, 0xFF00AAAA,
            0xFFAA0000, 0xFFAA00AA, 0xFFAA5500, 0xFFAAAAAA,
            0xFF555555, 0xFF5555FF, 0xFF55FF55, 0xFF55FFFF,
            0xFFFF5555, 0xFFFF55FF, 0xFFFFFF55, 0xFFFFFFFF,
    };

    private Palette() {
    }

    static int foreground(int attribute) {
        return COLORS[attribute & 0x0F];
    }

    static int background(int attribute, boolean blink) {
        int mask = blink ? 0x07 : 0x0F;
        return COLORS[(attribute >> 4) & mask];
    }
}
