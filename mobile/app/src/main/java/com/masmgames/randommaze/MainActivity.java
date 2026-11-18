package com.masmgames.randommaze;

import android.app.Activity;
import android.content.pm.ActivityInfo;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.Typeface;
import android.os.Bundle;
import android.os.Handler;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.Button;
import android.widget.GridLayout;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.widget.Toast;
import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;

public final class MainActivity extends Activity {
    static { System.loadLibrary("dosbox"); }

    private native long nativeStart(String gameDirectory);
    private native void nativeSendKey(long handle, int scan, int ascii);
    private native byte[] nativeFrame(long handle);
    private native void nativeStop(long handle);

    private final Handler handler = new Handler();
    private long emulator;
    private MazeView screen;
    private Button playerFocus;
    private int touchPlayer;
    private boolean duelMode;
    private final Runnable refresh = new Runnable() {
        @Override public void run() {
            if (emulator != 0 && screen != null) {
                screen.setFrame(nativeFrame(emulator));
                handler.postDelayed(this, 40);
            }
        }
    };

    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE);
        getWindow().getDecorView().setSystemUiVisibility(
            View.SYSTEM_UI_FLAG_FULLSCREEN | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION |
            View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY | View.SYSTEM_UI_FLAG_LAYOUT_STABLE);
        try {
            File gameDir = new File(getFilesDir(), "game");
            if (!gameDir.exists() && !gameDir.mkdirs()) throw new IllegalStateException("Could not prepare game files");
            copyAsset("game/MAZE.ASM", new File(gameDir, "MAZE.ASM"));
            emulator = nativeStart(gameDir.getAbsolutePath());
            if (emulator == 0) throw new IllegalStateException("The DOS emulator could not start");
            setContentView(buildLayout());
            handler.post(refresh);
        } catch (Exception error) {
            Toast.makeText(this, error.getMessage(), Toast.LENGTH_LONG).show();
            finish();
        }
    }

    private void copyAsset(String asset, File destination) throws Exception {
        if (destination.isFile()) return;
        try (InputStream in = getAssets().open(asset); FileOutputStream out = new FileOutputStream(destination)) {
            byte[] buffer = new byte[8192];
            int count;
            while ((count = in.read(buffer)) != -1) out.write(buffer, 0, count);
        }
    }

    private View buildLayout() {
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.HORIZONTAL);
        root.setGravity(Gravity.CENTER);
        root.setBackgroundColor(Color.rgb(12, 17, 13));
        screen = new MazeView();
        root.addView(screen, new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT, 1f));

        LinearLayout panel = new LinearLayout(this);
        panel.setOrientation(LinearLayout.VERTICAL);
        panel.setGravity(Gravity.CENTER);
        panel.setPadding(dp(8), dp(7), dp(8), dp(7));
        panel.setBackgroundColor(Color.rgb(16, 24, 18));
        root.addView(panel, new LinearLayout.LayoutParams(dp(226), ViewGroup.LayoutParams.MATCH_PARENT));

        LinearLayout modes = new LinearLayout(this);
        addModeButton(modes, "1 PLAYER", 0x02, '1', false);
        addModeButton(modes, "2 PLAYERS", 0x03, '2', true);
        panel.addView(modes, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(48)));

        GridLayout pad = new GridLayout(this);
        pad.setColumnCount(3);
        pad.setRowCount(3);
        pad.setAlignmentMode(GridLayout.ALIGN_BOUNDS);
        panel.addView(pad, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(174)));
        addGridKey(pad, "↑", 0, 1, 0x48, 0);
        addGridKey(pad, "←", 1, 0, 0x4B, 0);
        addGridKey(pad, "P1", 1, 1, -1, -1);
        addGridKey(pad, "→", 1, 2, 0x4D, 0);
        addGridKey(pad, "↓", 2, 1, 0x50, 0);

        LinearLayout actions = new LinearLayout(this);
        addButton(actions, "HINT", 0x23, 'h', 1f);
        addButton(actions, "MENU", 0x01, 27, 1f);
        panel.addView(actions, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(48)));
        Button enter = makeButton("ENTER");
        enter.setOnClickListener(v -> sendKey(0x1C, 13));
        panel.addView(enter, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(46)));
        TextView caption = new TextView(this);
        caption.setText("MASM GAME · TOUCH CONTROLS");
        caption.setTextColor(Color.rgb(133, 151, 132));
        caption.setTextSize(9);
        caption.setGravity(Gravity.CENTER);
        panel.addView(caption, new LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, dp(28)));
        return root;
    }

    private void addGridKey(GridLayout grid, String label, int row, int col, int scan, int ascii) {
        Button button = makeButton(label);
        if (scan == -1) {
            playerFocus = button;
            button.setText("P1");
            button.setOnClickListener(v -> {
                if (duelMode) touchPlayer = 1 - touchPlayer;
                button.setText(touchPlayer == 0 ? "P1" : "P2");
            });
        } else {
            button.setOnClickListener(v -> sendDirection(scan, ascii));
        }
        GridLayout.LayoutParams lp = new GridLayout.LayoutParams(GridLayout.spec(row, 1f), GridLayout.spec(col, 1f));
        lp.width = 0;
        lp.height = 0;
        lp.setMargins(dp(3), dp(3), dp(3), dp(3));
        grid.addView(button, lp);
    }

    private void addButton(LinearLayout row, String label, int scan, int ascii, float weight) {
        Button button = makeButton(label);
        button.setOnClickListener(v -> sendKey(scan, ascii));
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT, weight);
        lp.setMargins(dp(3), dp(2), dp(3), dp(2));
        row.addView(button, lp);
    }

    private void addModeButton(LinearLayout row, String label, int scan, int ascii, boolean duel) {
        Button button = makeButton(label);
        button.setOnClickListener(v -> {
            duelMode = duel;
            touchPlayer = 0;
            if (playerFocus != null) playerFocus.setText("P1");
            sendKey(scan, ascii);
        });
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT, 1f);
        lp.setMargins(dp(3), dp(2), dp(3), dp(2));
        row.addView(button, lp);
    }

    private Button makeButton(String label) {
        Button button = new Button(this);
        button.setText(label);
        button.setTextSize(13);
        button.setTextColor(Color.rgb(225, 238, 214));
        button.setAllCaps(false);
        button.setPadding(0, 0, 0, 0);
        button.setBackgroundTintList(android.content.res.ColorStateList.valueOf(Color.rgb(38, 53, 40)));
        return button;
    }

    private int dp(int value) { return Math.round(value * getResources().getDisplayMetrics().density); }
    private void sendKey(int scan, int ascii) { if (emulator != 0) nativeSendKey(emulator, scan, ascii); }
    private void sendDirection(int arrowScan, int arrowAscii) {
        if (touchPlayer == 0) {
            switch (arrowScan) {
                case 0x48: sendKey(0x11, 'w'); break;
                case 0x50: sendKey(0x1F, 's'); break;
                case 0x4B: sendKey(0x1E, 'a'); break;
                case 0x4D: sendKey(0x20, 'd'); break;
            }
        } else sendKey(arrowScan, arrowAscii);
    }

    @Override public boolean dispatchKeyEvent(android.view.KeyEvent event) {
        if (event.getAction() != android.view.KeyEvent.ACTION_DOWN) return super.dispatchKeyEvent(event);
        switch (event.getKeyCode()) {
            case android.view.KeyEvent.KEYCODE_DPAD_UP: sendKey(0x48, 0); return true;
            case android.view.KeyEvent.KEYCODE_DPAD_DOWN: sendKey(0x50, 0); return true;
            case android.view.KeyEvent.KEYCODE_DPAD_LEFT: sendKey(0x4B, 0); return true;
            case android.view.KeyEvent.KEYCODE_DPAD_RIGHT: sendKey(0x4D, 0); return true;
            case android.view.KeyEvent.KEYCODE_ENTER: sendKey(0x1C, 13); return true;
            case android.view.KeyEvent.KEYCODE_ESCAPE: sendKey(0x01, 27); return true;
            case android.view.KeyEvent.KEYCODE_H: sendKey(0x23, 'h'); return true;
            case android.view.KeyEvent.KEYCODE_W: sendKey(0x11, 'w'); return true;
            case android.view.KeyEvent.KEYCODE_A: sendKey(0x1E, 'a'); return true;
            case android.view.KeyEvent.KEYCODE_S: sendKey(0x1F, 's'); return true;
            case android.view.KeyEvent.KEYCODE_D: sendKey(0x20, 'd'); return true;
            default:
                int unicode = event.getUnicodeChar();
                if (unicode == '1' || unicode == '2') { sendKey(unicode == '1' ? 0x02 : 0x03, unicode); return true; }
                return super.dispatchKeyEvent(event);
        }
    }

    @Override public void onBackPressed() { sendKey(0x01, 27); }

    @Override protected void onDestroy() {
        handler.removeCallbacks(refresh);
        if (emulator != 0) { nativeStop(emulator); emulator = 0; }
        super.onDestroy();
    }

    private final class MazeView extends View {
        private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
        private byte[] frame = new byte[80 * 25 * 2];
        private final int[] colors = {
            Color.rgb(0,0,0), Color.rgb(0,0,170), Color.rgb(0,170,0), Color.rgb(0,170,170),
            Color.rgb(170,0,0), Color.rgb(170,0,170), Color.rgb(170,85,0), Color.rgb(170,170,170),
            Color.rgb(85,85,85), Color.rgb(85,85,255), Color.rgb(85,255,85), Color.rgb(85,255,255),
            Color.rgb(255,85,85), Color.rgb(255,85,255), Color.rgb(255,255,85), Color.rgb(255,255,255)
        };

        MazeView() { super(MainActivity.this); setLayerType(View.LAYER_TYPE_SOFTWARE, null); }
        void setFrame(byte[] current) { if (current != null && current.length == 4000) { frame = current; invalidate(); } }

        @Override protected void onMeasure(int widthSpec, int heightSpec) {
            int maxW = MeasureSpec.getSize(widthSpec), maxH = MeasureSpec.getSize(heightSpec);
            int width = Math.min(maxW, Math.round(maxH * 1.6f));
            int height = Math.round(width / 1.6f);
            setMeasuredDimension(width, Math.min(height, maxH));
        }

        @Override protected void onDraw(Canvas canvas) {
            super.onDraw(canvas);
            float cw = getWidth() / 80f, ch = getHeight() / 25f;
            paint.setTypeface(Typeface.MONOSPACE);
            for (int row=0; row<25; row++) for (int col=0; col<80; col++) {
                int i = (row * 80 + col) * 2;
                int code = frame[i] & 255, attr = frame[i+1] & 255;
                float x=col*cw, y=row*ch;
                paint.setColor(colors[(attr >>> 4) & 15]);
                canvas.drawRect(x, y, x+cw+1, y+ch+1, paint);
                if (code == 0xDF) {
                    paint.setColor(colors[attr & 15]);
                    canvas.drawRect(x, y, x+cw+1, y+ch/2f+1, paint);
                } else if (code >= 0x20 && code < 0x7F) {
                    paint.setColor(colors[attr & 15]);
                    paint.setTextSize(ch * .82f);
                    canvas.drawText(String.valueOf((char)code), x, y+ch*.82f, paint);
                }
            }
        }
    }
}
