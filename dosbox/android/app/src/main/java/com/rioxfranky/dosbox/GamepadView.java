package com.rioxfranky.dosbox;

import android.content.Context;
import android.graphics.Typeface;
import android.view.Gravity;
import android.widget.GridLayout;
import android.widget.LinearLayout;
import android.widget.TextView;

final class GamepadView extends LinearLayout {
    private final float density;

    GamepadView(Context context) {
        super(context);
        density = context.getResources().getDisplayMetrics().density;
        setOrientation(HORIZONTAL);
        setGravity(Gravity.CENTER);
        int padding = dp(6);
        setPadding(padding, padding, padding, padding);
        addView(pad(context, "P1  W A S D", 0xFF55FF55,
                () -> letter('w', 0x11), () -> letter('a', 0x1E), () -> letter('s', 0x1F), () -> letter('d', 0x20)),
                new LayoutParams(0, LayoutParams.WRAP_CONTENT, 1f));
        addView(middle(context), new LayoutParams(LayoutParams.WRAP_CONTENT, LayoutParams.WRAP_CONTENT));
        addView(pad(context, "P2  ARROWS", 0xFF55FFFF,
                () -> KeyMapper.sendSpecial(KeyMapper.SCAN_UP, 0),
                () -> KeyMapper.sendSpecial(KeyMapper.SCAN_LEFT, 0),
                () -> KeyMapper.sendSpecial(KeyMapper.SCAN_DOWN, 0),
                () -> KeyMapper.sendSpecial(KeyMapper.SCAN_RIGHT, 0)),
                new LayoutParams(0, LayoutParams.WRAP_CONTENT, 1f));
    }

    private int dp(float value) {
        return Math.round(value * density);
    }

    private static void letter(char character, int scan) {
        NativeBridge.sendKey(scan, character);
    }

    private LinearLayout pad(Context context, String title, int color, Runnable up, Runnable left, Runnable down, Runnable right) {
        LinearLayout column = new LinearLayout(context);
        column.setOrientation(VERTICAL);
        column.setGravity(Gravity.CENTER_HORIZONTAL);
        TextView label = new TextView(context);
        label.setText(title);
        label.setTextColor(color);
        label.setTypeface(Typeface.MONOSPACE, Typeface.BOLD);
        label.setGravity(Gravity.CENTER);
        column.addView(label);
        GridLayout grid = new GridLayout(context);
        grid.setColumnCount(3);
        grid.setRowCount(3);
        int size = dp(54);
        addCell(grid, null, 0, 0, size);
        addCell(grid, new KeyButton(context, "▲", true, up), 0, 1, size);
        addCell(grid, null, 0, 2, size);
        addCell(grid, new KeyButton(context, "◄", true, left), 1, 0, size);
        addCell(grid, null, 1, 1, size);
        addCell(grid, new KeyButton(context, "►", true, right), 1, 2, size);
        addCell(grid, null, 2, 0, size);
        addCell(grid, new KeyButton(context, "▼", true, down), 2, 1, size);
        addCell(grid, null, 2, 2, size);
        column.addView(grid);
        return column;
    }

    private void addCell(GridLayout grid, KeyButton button, int row, int column, int size) {
        GridLayout.LayoutParams params = new GridLayout.LayoutParams(GridLayout.spec(row), GridLayout.spec(column));
        params.width = size;
        params.height = size;
        int margin = dp(2);
        params.setMargins(margin, margin, margin, margin);
        if (button == null) {
            TextView spacer = new TextView(grid.getContext());
            grid.addView(spacer, params);
        } else {
            grid.addView(button, params);
        }
    }

    private LinearLayout middle(Context context) {
        LinearLayout column = new LinearLayout(context);
        column.setOrientation(VERTICAL);
        column.setGravity(Gravity.CENTER);
        int width = dp(76);
        int height = dp(36);
        String[] labels = {"ESC", "ENTER", "H", "1", "2"};
        Runnable[] actions = {
                () -> KeyMapper.sendSpecial(KeyMapper.SCAN_ESC, 27),
                KeyMapper::sendEnter,
                () -> NativeBridge.sendKey(0x23, 'h'),
                () -> NativeBridge.sendKey(0x02, '1'),
                () -> NativeBridge.sendKey(0x03, '2'),
        };
        for (int index = 0; index < labels.length; index++) {
            LayoutParams params = new LayoutParams(width, height);
            params.setMargins(dp(4), dp(3), dp(4), dp(3));
            column.addView(new KeyButton(context, labels[index], false, actions[index]), params);
        }
        return column;
    }
}
