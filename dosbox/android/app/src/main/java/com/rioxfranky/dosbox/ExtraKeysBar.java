package com.rioxfranky.dosbox;

import android.content.Context;
import android.widget.LinearLayout;

final class ExtraKeysBar extends LinearLayout {
    interface Actions {
        void toggleKeyboard();

        void toggleGamepad();
    }

    private final KeyButton ctrl;

    ExtraKeysBar(Context context, Actions actions) {
        super(context);
        setOrientation(HORIZONTAL);
        int gap = Math.round(context.getResources().getDisplayMetrics().density * 3);
        setPadding(gap, gap, gap, gap);
        add(new KeyButton(context, "ESC", false, () -> KeyMapper.sendSpecial(KeyMapper.SCAN_ESC, 27)), gap);
        add(new KeyButton(context, "TAB", false, () -> KeyMapper.sendSpecial(KeyMapper.SCAN_TAB, 9)), gap);
        ctrl = new KeyButton(context, "CTRL", false, () -> KeyMapper.setStickyCtrl(!KeyMapper.isStickyCtrl()));
        add(ctrl, gap);
        add(new KeyButton(context, "←", true, () -> KeyMapper.sendSpecial(KeyMapper.SCAN_LEFT, 0)), gap);
        add(new KeyButton(context, "↑", true, () -> KeyMapper.sendSpecial(KeyMapper.SCAN_UP, 0)), gap);
        add(new KeyButton(context, "↓", true, () -> KeyMapper.sendSpecial(KeyMapper.SCAN_DOWN, 0)), gap);
        add(new KeyButton(context, "→", true, () -> KeyMapper.sendSpecial(KeyMapper.SCAN_RIGHT, 0)), gap);
        add(new KeyButton(context, "↵", false, KeyMapper::sendEnter), gap);
        add(new KeyButton(context, "⌨", false, actions::toggleKeyboard), gap);
        add(new KeyButton(context, "🎮", false, actions::toggleGamepad), gap);
        KeyMapper.setCtrlListener(ctrl::setActive);
    }

    private void add(KeyButton button, int gap) {
        int height = Math.round(getResources().getDisplayMetrics().density * 42);
        LayoutParams params = new LayoutParams(0, height, 1f);
        params.setMargins(gap, 0, gap, 0);
        addView(button, params);
    }
}
