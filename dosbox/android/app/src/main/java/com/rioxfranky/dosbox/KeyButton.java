package com.rioxfranky.dosbox;

import android.annotation.SuppressLint;
import android.content.Context;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.os.Handler;
import android.os.Looper;
import android.view.Gravity;
import android.view.HapticFeedbackConstants;
import android.view.MotionEvent;
import android.widget.TextView;

final class KeyButton extends TextView {
    private static final long FIRST_REPEAT_MS = 320;
    private static final long REPEAT_MS = 85;
    private static final int IDLE_COLOR = 0xFF2A2A3A;
    private static final int PRESSED_COLOR = 0xFF5555FF;
    private static final int ACTIVE_COLOR = 0xFF00AA00;

    private final Handler handler = new Handler(Looper.getMainLooper());
    private final GradientDrawable shape = new GradientDrawable();
    private final Runnable action;
    private final boolean repeats;
    private boolean active;
    private final Runnable repeater = new Runnable() {
        @Override
        public void run() {
            action.run();
            handler.postDelayed(this, REPEAT_MS);
        }
    };

    @SuppressLint("ClickableViewAccessibility")
    KeyButton(Context context, String label, boolean repeats, Runnable action) {
        super(context);
        this.action = action;
        this.repeats = repeats;
        setText(label);
        setTextColor(0xFFFFFFFF);
        setTypeface(Typeface.MONOSPACE, Typeface.BOLD);
        setGravity(Gravity.CENTER);
        setTextSize(14);
        shape.setCornerRadius(context.getResources().getDisplayMetrics().density * 6);
        shape.setColor(IDLE_COLOR);
        setBackground(shape);
        setOnTouchListener((view, event) -> {
            switch (event.getActionMasked()) {
                case MotionEvent.ACTION_DOWN:
                    shape.setColor(PRESSED_COLOR);
                    performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP);
                    action.run();
                    if (this.repeats) {
                        handler.postDelayed(repeater, FIRST_REPEAT_MS);
                    }
                    return true;
                case MotionEvent.ACTION_UP:
                case MotionEvent.ACTION_CANCEL:
                    handler.removeCallbacks(repeater);
                    shape.setColor(active ? ACTIVE_COLOR : IDLE_COLOR);
                    if (event.getActionMasked() == MotionEvent.ACTION_UP) {
                        performClick();
                    }
                    return true;
                default:
                    return true;
            }
        });
    }

    @Override
    public boolean performClick() {
        return super.performClick();
    }

    void setActive(boolean active) {
        this.active = active;
        shape.setColor(active ? ACTIVE_COLOR : IDLE_COLOR);
    }
}
