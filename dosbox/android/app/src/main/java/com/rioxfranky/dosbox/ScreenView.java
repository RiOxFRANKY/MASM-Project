package com.rioxfranky.dosbox;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.RectF;
import android.graphics.Typeface;
import android.os.Build;
import android.text.InputType;
import android.view.Choreographer;
import android.view.GestureDetector;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.ScaleGestureDetector;
import android.view.View;
import android.view.WindowInsets;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;

final class ScreenView extends View implements Choreographer.FrameCallback {
    interface SessionListener {
        void onSessionEnded();
    }

    private static final int CELL_WIDTH = 16;
    private static final int CELL_HEIGHT = 32;
    private static final int IMAGE_WIDTH = NativeBridge.COLUMNS * CELL_WIDTH;
    private static final int IMAGE_HEIGHT = NativeBridge.ROWS * CELL_HEIGHT;
    private static final float MAX_ZOOM = 4f;
    private static final float DOUBLE_TAP_ZOOM = 2.2f;
    private static final long CURSOR_BLINK_NANOS = 266_000_000L;
    private static final int FRAME_COLOR = 0xFF2A2A2A;

    private final byte[] screen = new byte[NativeBridge.SCREEN_BYTES];
    private final byte[] drawn = new byte[NativeBridge.CELL_BYTES];
    private final Bitmap image = Bitmap.createBitmap(IMAGE_WIDTH, IMAGE_HEIGHT, Bitmap.Config.ARGB_8888);
    private final Canvas imageCanvas = new Canvas(image);
    private final Paint fill = new Paint();
    private final Paint text = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint blit = new Paint(Paint.FILTER_BITMAP_FLAG | Paint.ANTI_ALIAS_FLAG);
    private final RectF target = new RectF();
    private final ScaleGestureDetector scaleDetector;
    private final GestureDetector gestureDetector;
    private SessionListener sessionListener;
    private int version = -1;
    private boolean imageValid;
    private boolean drawnBlink;
    private boolean cursorOn = true;
    private long lastBlink;
    private boolean wasRunning;
    private boolean keyboardRequested;
    private float textBaseline;
    private float zoom = 1f;
    private float panX;
    private float panY;

    ScreenView(Context context) {
        super(context);
        setFocusable(true);
        setFocusableInTouchMode(true);
        setBackgroundColor(FRAME_COLOR);
        text.setTypeface(Typeface.MONOSPACE);
        text.setTextSize(CELL_HEIGHT * 0.8f);
        float glyphWidth = text.measureText("M");
        if (glyphWidth > 0) {
            text.setTextScaleX(CELL_WIDTH * 0.96f / glyphWidth);
        }
        Paint.FontMetrics metrics = text.getFontMetrics();
        textBaseline = (CELL_HEIGHT - (metrics.descent - metrics.ascent)) / 2f - metrics.ascent;
        imageCanvas.drawColor(0xFF000000);
        scaleDetector = new ScaleGestureDetector(context, new ScaleGestureDetector.SimpleOnScaleGestureListener() {
            @Override
            public boolean onScale(ScaleGestureDetector detector) {
                zoomAround(zoom * detector.getScaleFactor(), detector.getFocusX(), detector.getFocusY());
                return true;
            }
        });
        gestureDetector = new GestureDetector(context, new GestureDetector.SimpleOnGestureListener() {
            @Override
            public boolean onDown(MotionEvent event) {
                return true;
            }

            @Override
            public boolean onScroll(MotionEvent first, MotionEvent event, float distanceX, float distanceY) {
                panX -= distanceX;
                panY -= distanceY;
                invalidate();
                return true;
            }

            @Override
            public boolean onDoubleTap(MotionEvent event) {
                if (zoom > 1.05f) {
                    zoom = 1f;
                    panX = 0f;
                    panY = 0f;
                    invalidate();
                } else {
                    zoomAround(DOUBLE_TAP_ZOOM, event.getX(), event.getY());
                }
                return true;
            }
        });
    }

    void setSessionListener(SessionListener listener) {
        sessionListener = listener;
    }

    @Override
    protected void onAttachedToWindow() {
        super.onAttachedToWindow();
        Choreographer.getInstance().postFrameCallback(this);
    }

    @Override
    protected void onDetachedFromWindow() {
        Choreographer.getInstance().removeFrameCallback(this);
        super.onDetachedFromWindow();
    }

    @Override
    public void doFrame(long frameTimeNanos) {
        int current = NativeBridge.readScreen(screen);
        boolean changed = false;
        if (current != version) {
            version = current;
            renderImage();
            changed = true;
        }
        if (frameTimeNanos - lastBlink > CURSOR_BLINK_NANOS) {
            lastBlink = frameTimeNanos;
            cursorOn = !cursorOn;
            changed = true;
        }
        if (changed) {
            invalidate();
        }
        boolean running = NativeBridge.isRunning();
        if (wasRunning && !running && sessionListener != null) {
            sessionListener.onSessionEnded();
        }
        wasRunning = running;
        Choreographer.getInstance().postFrameCallback(this);
    }

    private void renderImage() {
        boolean blink = screen[NativeBridge.CELL_BYTES + 3] != 0;
        boolean everything = !imageValid || blink != drawnBlink;
        for (int cell = 0; cell < NativeBridge.COLUMNS * NativeBridge.ROWS; cell++) {
            int index = cell * 2;
            if (!everything && drawn[index] == screen[index] && drawn[index + 1] == screen[index + 1]) {
                continue;
            }
            drawCell(cell % NativeBridge.COLUMNS, cell / NativeBridge.COLUMNS, screen[index] & 0xFF, screen[index + 1] & 0xFF, blink);
            drawn[index] = screen[index];
            drawn[index + 1] = screen[index + 1];
        }
        drawnBlink = blink;
        imageValid = true;
    }

    private void drawCell(int column, int row, int character, int attribute, boolean blink) {
        int left = column * CELL_WIDTH;
        int top = row * CELL_HEIGHT;
        int right = left + CELL_WIDTH;
        int bottom = top + CELL_HEIGHT;
        int middleY = top + CELL_HEIGHT / 2;
        int middleX = left + CELL_WIDTH / 2;
        int foreground = Palette.foreground(attribute);
        fill.setColor(Palette.background(attribute, blink));
        imageCanvas.drawRect(left, top, right, bottom, fill);
        fill.setColor(foreground);
        switch (character) {
            case 0x00:
            case 0x20:
            case 0xFF:
                return;
            case 0xDB:
                imageCanvas.drawRect(left, top, right, bottom, fill);
                return;
            case 0xDF:
                imageCanvas.drawRect(left, top, right, middleY, fill);
                return;
            case 0xDC:
                imageCanvas.drawRect(left, middleY, right, bottom, fill);
                return;
            case 0xDD:
                imageCanvas.drawRect(left, top, middleX, bottom, fill);
                return;
            case 0xDE:
                imageCanvas.drawRect(middleX, top, right, bottom, fill);
                return;
            default:
                text.setColor(foreground);
                imageCanvas.save();
                imageCanvas.clipRect(left, top, right, bottom);
                imageCanvas.drawText(Cp437.glyph(character), left + CELL_WIDTH * 0.02f, top + textBaseline, text);
                imageCanvas.restore();
        }
    }

    private float baseScale() {
        return Math.min(getWidth() / (float) IMAGE_WIDTH, getHeight() / (float) IMAGE_HEIGHT);
    }

    private void clampPan() {
        float scale = baseScale() * zoom;
        float spareX = Math.max(0f, (IMAGE_WIDTH * scale - getWidth()) / 2f);
        float spareY = Math.max(0f, (IMAGE_HEIGHT * scale - getHeight()) / 2f);
        panX = Math.max(-spareX, Math.min(spareX, panX));
        panY = Math.max(-spareY, Math.min(spareY, panY));
    }

    private void zoomAround(float requested, float focusX, float focusY) {
        float next = Math.max(1f, Math.min(MAX_ZOOM, requested));
        float ratio = next / zoom;
        float offsetX = focusX - getWidth() / 2f;
        float offsetY = focusY - getHeight() / 2f;
        panX = offsetX - (offsetX - panX) * ratio;
        panY = offsetY - (offsetY - panY) * ratio;
        zoom = next;
        invalidate();
    }

    @Override
    protected void onDraw(Canvas canvas) {
        clampPan();
        float scale = baseScale() * zoom;
        float width = IMAGE_WIDTH * scale;
        float height = IMAGE_HEIGHT * scale;
        float left = (getWidth() - width) / 2f + panX;
        float top = (getHeight() - height) / 2f + panY;
        target.set(left, top, left + width, top + height);
        canvas.drawBitmap(image, null, target, blit);
        drawCursor(canvas, left, top, scale);
    }

    private void drawCursor(Canvas canvas, float left, float top, float scale) {
        boolean visible = screen[NativeBridge.CELL_BYTES + 2] != 0;
        if (!visible || !cursorOn) {
            return;
        }
        int row = Math.min(screen[NativeBridge.CELL_BYTES] & 0xFF, NativeBridge.ROWS - 1);
        int column = Math.min(screen[NativeBridge.CELL_BYTES + 1] & 0xFF, NativeBridge.COLUMNS - 1);
        int attribute = screen[(row * NativeBridge.COLUMNS + column) * 2 + 1] & 0xFF;
        float cellLeft = left + column * CELL_WIDTH * scale;
        float cellBottom = top + (row + 1) * CELL_HEIGHT * scale;
        fill.setColor(Palette.foreground(attribute));
        canvas.drawRect(cellLeft, cellBottom - CELL_HEIGHT * scale / 7f, cellLeft + CELL_WIDTH * scale, cellBottom, fill);
    }

    @Override
    public boolean onTouchEvent(MotionEvent event) {
        scaleDetector.onTouchEvent(event);
        gestureDetector.onTouchEvent(event);
        if (event.getActionMasked() == MotionEvent.ACTION_UP) {
            performClick();
        }
        return true;
    }

    @Override
    public boolean performClick() {
        return super.performClick();
    }

    void showKeyboard() {
        requestFocus();
        InputMethodManager manager = (InputMethodManager) getContext().getSystemService(Context.INPUT_METHOD_SERVICE);
        if (manager != null) {
            manager.showSoftInput(this, InputMethodManager.SHOW_IMPLICIT);
            keyboardRequested = true;
        }
    }

    void toggleKeyboard() {
        InputMethodManager manager = (InputMethodManager) getContext().getSystemService(Context.INPUT_METHOD_SERVICE);
        if (manager == null) {
            return;
        }
        if (keyboardVisible()) {
            manager.hideSoftInputFromWindow(getWindowToken(), 0);
            keyboardRequested = false;
        } else {
            showKeyboard();
        }
    }

    private boolean keyboardVisible() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            WindowInsets insets = getRootWindowInsets();
            if (insets != null) {
                return insets.isVisible(WindowInsets.Type.ime());
            }
        }
        return keyboardRequested;
    }

    @Override
    public boolean onCheckIsTextEditor() {
        return true;
    }

    @Override
    public InputConnection onCreateInputConnection(EditorInfo outAttrs) {
        outAttrs.inputType = InputType.TYPE_NULL;
        outAttrs.imeOptions = EditorInfo.IME_FLAG_NO_FULLSCREEN | EditorInfo.IME_FLAG_NO_EXTRACT_UI | EditorInfo.IME_ACTION_NONE;
        return new DosInputConnection(this);
    }

    @Override
    public boolean onKeyDown(int keyCode, KeyEvent event) {
        if (KeyMapper.handle(event)) {
            return true;
        }
        return super.onKeyDown(keyCode, event);
    }
}
