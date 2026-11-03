package com.rioxfranky.dosbox;

import android.annotation.SuppressLint;
import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.Path;
import android.graphics.RectF;
import android.os.Handler;
import android.os.Looper;
import android.view.HapticFeedbackConstants;
import android.view.MotionEvent;
import android.view.View;

final class DpadView extends View {
    private static final int NONE = -1;
    private static final int UP = 0;
    private static final int RIGHT = 1;
    private static final int DOWN = 2;
    private static final int LEFT = 3;
    private static final int[] SCANS = {KeyMapper.SCAN_UP, KeyMapper.SCAN_RIGHT, KeyMapper.SCAN_DOWN, KeyMapper.SCAN_LEFT};
    private static final long FIRST_REPEAT_MS = 300;
    private static final long REPEAT_MS = 90;

    private final Paint fill = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint stroke = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint arrow = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Path path = new Path();
    private final RectF bounds = new RectF();
    private final Handler handler = new Handler(Looper.getMainLooper());
    private int direction = NONE;
    private final Runnable repeater = new Runnable() {
        @Override
        public void run() {
            if (direction != NONE) {
                KeyMapper.sendSpecial(SCANS[direction], 0);
                handler.postDelayed(this, REPEAT_MS);
            }
        }
    };

    DpadView(Context context) {
        super(context);
        float density = context.getResources().getDisplayMetrics().density;
        stroke.setStyle(Paint.Style.STROKE);
        stroke.setStrokeWidth(2 * density);
        stroke.setColor(KeyButton.STROKE);
        fill.setStyle(Paint.Style.FILL);
        arrow.setStyle(Paint.Style.FILL);
        arrow.setColor(0xFF8E8E8E);
    }

    @Override
    protected void onDraw(Canvas canvas) {
        float centerX = getWidth() / 2f;
        float centerY = getHeight() / 2f;
        float outer = Math.min(centerX, centerY) - stroke.getStrokeWidth();
        float inner = outer * 0.40f;
        fill.setColor(0xFF242424);
        canvas.drawCircle(centerX, centerY, outer, fill);
        if (direction != NONE) {
            fill.setColor(KeyButton.PRESSED);
            bounds.set(centerX - outer, centerY - outer, centerX + outer, centerY + outer);
            canvas.drawArc(bounds, direction * 90f - 135f, 90f, true, fill);
        }
        canvas.drawCircle(centerX, centerY, outer, stroke);
        for (int index = 0; index < 4; index++) {
            double angle = Math.toRadians(45 + index * 90);
            float cos = (float) Math.cos(angle);
            float sin = (float) Math.sin(angle);
            canvas.drawLine(centerX + cos * inner, centerY + sin * inner, centerX + cos * outer, centerY + sin * outer, stroke);
        }
        fill.setColor(0xFF1A1A1A);
        canvas.drawCircle(centerX, centerY, inner, fill);
        canvas.drawCircle(centerX, centerY, inner, stroke);
        float distance = (inner + outer) / 2f;
        float size = outer * 0.13f;
        for (int index = 0; index < 4; index++) {
            double angle = Math.toRadians(index * 90 - 90);
            float cos = (float) Math.cos(angle);
            float sin = (float) Math.sin(angle);
            float tipX = centerX + cos * (distance + size);
            float tipY = centerY + sin * (distance + size);
            float baseX = centerX + cos * (distance - size * 0.6f);
            float baseY = centerY + sin * (distance - size * 0.6f);
            path.reset();
            path.moveTo(tipX, tipY);
            path.lineTo(baseX - sin * size, baseY + cos * size);
            path.lineTo(baseX + sin * size, baseY - cos * size);
            path.close();
            canvas.drawPath(path, arrow);
        }
    }

    private int directionAt(float x, float y) {
        float dx = x - getWidth() / 2f;
        float dy = y - getHeight() / 2f;
        float outer = Math.min(getWidth(), getHeight()) / 2f;
        if (Math.hypot(dx, dy) < outer * 0.22f) {
            return NONE;
        }
        if (Math.abs(dx) > Math.abs(dy)) {
            return dx > 0 ? RIGHT : LEFT;
        }
        return dy > 0 ? DOWN : UP;
    }

    private void press(int next) {
        if (next == direction) {
            return;
        }
        handler.removeCallbacks(repeater);
        direction = next;
        if (next != NONE) {
            performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY);
            KeyMapper.sendSpecial(SCANS[next], 0);
            handler.postDelayed(repeater, FIRST_REPEAT_MS);
        }
        invalidate();
    }

    @SuppressLint("ClickableViewAccessibility")
    @Override
    public boolean onTouchEvent(MotionEvent event) {
        switch (event.getActionMasked()) {
            case MotionEvent.ACTION_DOWN:
            case MotionEvent.ACTION_MOVE:
                press(directionAt(event.getX(), event.getY()));
                return true;
            case MotionEvent.ACTION_UP:
            case MotionEvent.ACTION_CANCEL:
                press(NONE);
                return true;
            default:
                return true;
        }
    }
}
