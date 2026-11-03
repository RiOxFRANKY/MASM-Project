package com.rioxfranky.dosbox;

import android.annotation.SuppressLint;
import android.content.Context;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.os.Handler;
import android.os.Looper;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.HapticFeedbackConstants;
import android.view.MotionEvent;
import android.widget.TextView;

final class KeyButton extends TextView {
    enum Shape { PILL, CIRCLE }

    static final int FILL = 0xFF2C2C2C;
    static final int PRESSED = 0xFF5A5A5A;
    static final int STROKE = 0xFF9A9A9A;
    static final int TEXT = 0xFFD2D2D2;

    private static final long FIRST_REPEAT_MS = 320;
    private static final long REPEAT_MS = 90;

    private final Handler handler = new Handler(Looper.getMainLooper());
    private final GradientDrawable background = new GradientDrawable();
    private final Shape shape;
    private final Runnable action;
    private final boolean repeats;
    private final Runnable repeater = new Runnable() {
        @Override
        public void run() {
            action.run();
            handler.postDelayed(this, REPEAT_MS);
        }
    };

    @SuppressLint("ClickableViewAccessibility")
    KeyButton(Context context, String label, Shape shape, boolean repeats, Runnable action) {
        super(context);
        this.shape = shape;
        this.action = action;
        this.repeats = repeats;
        setText(label);
        setTextColor(TEXT);
        setTypeface(Typeface.create("sans-serif-condensed", Typeface.BOLD));
        setGravity(Gravity.CENTER);
        setIncludeFontPadding(false);
        setSingleLine(true);
        background.setShape(shape == Shape.CIRCLE ? GradientDrawable.OVAL : GradientDrawable.RECTANGLE);
        background.setColor(FILL);
        background.setStroke(Math.round(context.getResources().getDisplayMetrics().density * 2), STROKE);
        setBackground(background);
        setOnTouchListener((view, event) -> {
            switch (event.getActionMasked()) {
                case MotionEvent.ACTION_DOWN:
                    background.setColor(PRESSED);
                    performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY);
                    this.action.run();
                    if (this.repeats) {
                        handler.postDelayed(repeater, FIRST_REPEAT_MS);
                    }
                    return true;
                case MotionEvent.ACTION_UP:
                case MotionEvent.ACTION_CANCEL:
                    handler.removeCallbacks(repeater);
                    background.setColor(FILL);
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

    @Override
    protected void onSizeChanged(int width, int height, int oldWidth, int oldHeight) {
        super.onSizeChanged(width, height, oldWidth, oldHeight);
        if (shape == Shape.PILL) {
            background.setCornerRadius(height / 2f);
            setTextSize(TypedValue.COMPLEX_UNIT_PX, height * 0.42f);
        } else {
            setTextSize(TypedValue.COMPLEX_UNIT_PX, height * 0.36f);
        }
    }
}
