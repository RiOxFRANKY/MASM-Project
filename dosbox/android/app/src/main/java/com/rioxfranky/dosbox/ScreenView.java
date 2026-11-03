package com.rioxfranky.dosbox;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.Typeface;
import android.os.Build;
import android.text.InputType;
import android.view.Choreographer;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.View;
import android.view.WindowInsets;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;

final class ScreenView extends View implements Choreographer.FrameCallback {
    interface SessionListener {
        void onSessionEnded();
    }

    private static final long CURSOR_BLINK_NANOS = 266_000_000L;

    private final byte[] screen = new byte[NativeBridge.SCREEN_BYTES];
    private final Paint fill = new Paint();
    private final Paint text = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final int[] columnEdges = new int[NativeBridge.COLUMNS + 1];
    private final int[] rowEdges = new int[NativeBridge.ROWS * 2 + 1];
    private SessionListener sessionListener;
    private int version = -1;
    private boolean cursorOn = true;
    private long lastBlink;
    private boolean wasRunning;
    private boolean keyboardRequested;
    private float textBaseline;

    ScreenView(Context context) {
        super(context);
        setFocusable(true);
        setFocusableInTouchMode(true);
        setBackgroundColor(0xFF000000);
        text.setTypeface(Typeface.MONOSPACE);
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
        boolean changed = current != version;
        version = current;
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

    @Override
    protected void onSizeChanged(int width, int height, int oldWidth, int oldHeight) {
        super.onSizeChanged(width, height, oldWidth, oldHeight);
        float pixel = Math.min(width / (float) NativeBridge.COLUMNS, height / (float) (NativeBridge.ROWS * 2));
        float left = (width - pixel * NativeBridge.COLUMNS) / 2f;
        float top = (height - pixel * NativeBridge.ROWS * 2) / 2f;
        for (int index = 0; index < columnEdges.length; index++) {
            columnEdges[index] = Math.round(left + index * pixel);
        }
        for (int index = 0; index < rowEdges.length; index++) {
            rowEdges[index] = Math.round(top + index * pixel);
        }
        float cellHeight = pixel * 2f;
        text.setTextSize(cellHeight * 0.82f);
        text.setTextScaleX(1f);
        float glyphWidth = text.measureText("M");
        if (glyphWidth > 0) {
            text.setTextScaleX(pixel * 0.98f / glyphWidth);
        }
        Paint.FontMetrics metrics = text.getFontMetrics();
        textBaseline = (cellHeight - (metrics.descent - metrics.ascent)) / 2f - metrics.ascent;
    }

    @Override
    protected void onDraw(Canvas canvas) {
        boolean blink = screen[NativeBridge.CELL_BYTES + 3] != 0;
        for (int row = 0; row < NativeBridge.ROWS; row++) {
            int top = rowEdges[row * 2];
            int middle = rowEdges[row * 2 + 1];
            int bottom = rowEdges[row * 2 + 2];
            for (int column = 0; column < NativeBridge.COLUMNS; column++) {
                int index = (row * NativeBridge.COLUMNS + column) * 2;
                int character = screen[index] & 0xFF;
                int attribute = screen[index + 1] & 0xFF;
                int left = columnEdges[column];
                int right = columnEdges[column + 1];
                int center = (left + right) / 2;
                int foreground = Palette.foreground(attribute);
                fill.setColor(Palette.background(attribute, blink));
                canvas.drawRect(left, top, right, bottom, fill);
                fill.setColor(foreground);
                switch (character) {
                    case 0x00:
                    case 0x20:
                    case 0xFF:
                        break;
                    case 0xDB:
                        canvas.drawRect(left, top, right, bottom, fill);
                        break;
                    case 0xDF:
                        canvas.drawRect(left, top, right, middle, fill);
                        break;
                    case 0xDC:
                        canvas.drawRect(left, middle, right, bottom, fill);
                        break;
                    case 0xDD:
                        canvas.drawRect(left, top, center, bottom, fill);
                        break;
                    case 0xDE:
                        canvas.drawRect(center, top, right, bottom, fill);
                        break;
                    default:
                        text.setColor(foreground);
                        canvas.drawText(Cp437.glyph(character), left, top + textBaseline, text);
                        break;
                }
            }
        }
        drawCursor(canvas);
    }

    private void drawCursor(Canvas canvas) {
        boolean visible = screen[NativeBridge.CELL_BYTES + 2] != 0;
        if (!visible || !cursorOn) {
            return;
        }
        int row = Math.min(screen[NativeBridge.CELL_BYTES] & 0xFF, NativeBridge.ROWS - 1);
        int column = Math.min(screen[NativeBridge.CELL_BYTES + 1] & 0xFF, NativeBridge.COLUMNS - 1);
        int attribute = screen[(row * NativeBridge.COLUMNS + column) * 2 + 1] & 0xFF;
        int top = rowEdges[row * 2];
        int bottom = rowEdges[row * 2 + 2];
        int thickness = Math.max(2, (bottom - top) / 7);
        fill.setColor(Palette.foreground(attribute));
        canvas.drawRect(columnEdges[column], bottom - thickness, columnEdges[column + 1], bottom, fill);
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
    public boolean onTouchEvent(MotionEvent event) {
        if (event.getAction() == MotionEvent.ACTION_UP) {
            showKeyboard();
            performClick();
        }
        return true;
    }

    @Override
    public boolean performClick() {
        return super.performClick();
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
