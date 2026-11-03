package com.rioxfranky.dosbox;

import android.content.Context;
import android.graphics.Rect;
import android.view.View;
import android.view.ViewGroup;

import java.util.ArrayList;
import java.util.List;

final class ControllerView extends ViewGroup {
    private static final float BOX_WIDTH = 431f;
    private static final float BOX_HEIGHT = 418f;

    private static final class Slot {
        final View view;
        final boolean circle;
        final float a;
        final float b;
        final float c;
        final float d;

        Slot(View view, boolean circle, float a, float b, float c, float d) {
            this.view = view;
            this.circle = circle;
            this.a = a;
            this.b = b;
            this.c = c;
            this.d = d;
        }
    }

    private final List<Slot> slots = new ArrayList<>();
    private final Rect rect = new Rect();

    ControllerView(Context context) {
        super(context);
        setBackgroundColor(0xFF000000);
    }

    void addRect(View view, float left, float top, float right, float bottom) {
        slots.add(new Slot(view, false, left, top, right, bottom));
        addView(view);
    }

    void addCircle(View view, float centerX, float centerY, float radius) {
        slots.add(new Slot(view, true, centerX, centerY, radius, 0));
        addView(view);
    }

    private void place(Slot slot, int width, int height, Rect out) {
        int usable = Math.max(1, height - getPaddingTop() - getPaddingBottom());
        float boxWidth = Math.min(width, usable * BOX_WIDTH / BOX_HEIGHT);
        float boxHeight = boxWidth * BOX_HEIGHT / BOX_WIDTH;
        float boxLeft = (width - boxWidth) / 2f;
        float boxTop = getPaddingTop() + usable - boxHeight;
        if (slot.circle) {
            float centerX = boxLeft + slot.a * boxWidth;
            float centerY = boxTop + slot.b * boxHeight;
            float radius = slot.c * boxWidth;
            out.set(Math.round(centerX - radius), Math.round(centerY - radius),
                    Math.round(centerX + radius), Math.round(centerY + radius));
        } else {
            out.set(Math.round(boxLeft + slot.a * boxWidth), Math.round(boxTop + slot.b * boxHeight),
                    Math.round(boxLeft + slot.c * boxWidth), Math.round(boxTop + slot.d * boxHeight));
        }
    }

    @Override
    protected void onMeasure(int widthMeasureSpec, int heightMeasureSpec) {
        int width = MeasureSpec.getSize(widthMeasureSpec);
        int height = Math.round(width * BOX_HEIGHT / BOX_WIDTH) + getPaddingTop() + getPaddingBottom();
        if (MeasureSpec.getMode(heightMeasureSpec) != MeasureSpec.UNSPECIFIED) {
            height = Math.min(height, MeasureSpec.getSize(heightMeasureSpec));
        }
        setMeasuredDimension(width, height);
        for (Slot slot : slots) {
            place(slot, width, height, rect);
            slot.view.measure(MeasureSpec.makeMeasureSpec(rect.width(), MeasureSpec.EXACTLY),
                    MeasureSpec.makeMeasureSpec(rect.height(), MeasureSpec.EXACTLY));
        }
    }

    @Override
    protected void onLayout(boolean changed, int left, int top, int right, int bottom) {
        for (Slot slot : slots) {
            place(slot, right - left, bottom - top, rect);
            slot.view.layout(rect.left, rect.top, rect.right, rect.bottom);
        }
    }
}
