package com.rioxfranky.dosbox;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.ClipData;
import android.content.Intent;
import android.content.SharedPreferences;
import android.net.Uri;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.Menu;
import android.view.MenuItem;
import android.view.View;
import android.widget.LinearLayout;
import android.widget.Toast;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;

public final class MainActivity extends Activity implements ExtraKeysBar.Actions, ScreenView.SessionListener {
    private static final int REQUEST_IMPORT = 1;
    private static final int REQUEST_EXPORT = 2;
    private static final int MENU_KEYBOARD = 1;
    private static final int MENU_GAMEPAD = 2;
    private static final int MENU_IMPORT = 3;
    private static final int MENU_EXPORT = 4;
    private static final int MENU_RESTART = 5;
    private static final String PREFERENCES = "dosbox";
    private static final String KEY_GAMEPAD = "gamepad";

    private final Handler handler = new Handler(Looper.getMainLooper());
    private DriveFiles drive;
    private ScreenView screen;
    private GamepadView gamepad;
    private String pendingExport;
    private boolean restarting;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        drive = new DriveFiles(this);
        drive.installStarterFiles();

        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setBackgroundColor(0xFF000000);
        screen = new ScreenView(this);
        screen.setSessionListener(this);
        gamepad = new GamepadView(this);
        root.addView(screen, new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, 0, 1f));
        root.addView(gamepad, new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT));
        root.addView(new ExtraKeysBar(this, this), new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT));
        setContentView(root);

        gamepad.setVisibility(preferences().getBoolean(KEY_GAMEPAD, false) ? View.VISIBLE : View.GONE);
        NativeBridge.start(drive.rootPathBytes());
        screen.requestFocus();
    }

    private SharedPreferences preferences() {
        return getSharedPreferences(PREFERENCES, MODE_PRIVATE);
    }

    @Override
    protected void onDestroy() {
        if (isFinishing()) {
            NativeBridge.stop();
        }
        super.onDestroy();
    }

    @Override
    public boolean onCreateOptionsMenu(Menu menu) {
        menu.add(Menu.NONE, MENU_KEYBOARD, 1, R.string.menu_keyboard).setShowAsAction(MenuItem.SHOW_AS_ACTION_IF_ROOM);
        menu.add(Menu.NONE, MENU_GAMEPAD, 2, R.string.menu_gamepad).setShowAsAction(MenuItem.SHOW_AS_ACTION_IF_ROOM);
        menu.add(Menu.NONE, MENU_IMPORT, 3, R.string.menu_import);
        menu.add(Menu.NONE, MENU_EXPORT, 4, R.string.menu_export);
        menu.add(Menu.NONE, MENU_RESTART, 5, R.string.menu_restart);
        return true;
    }

    @Override
    public boolean onOptionsItemSelected(MenuItem item) {
        switch (item.getItemId()) {
            case MENU_KEYBOARD:
                toggleKeyboard();
                return true;
            case MENU_GAMEPAD:
                toggleGamepad();
                return true;
            case MENU_IMPORT:
                startImport();
                return true;
            case MENU_EXPORT:
                startExport();
                return true;
            case MENU_RESTART:
                restartSession();
                return true;
            default:
                return super.onOptionsItemSelected(item);
        }
    }

    @Override
    public void toggleKeyboard() {
        screen.toggleKeyboard();
    }

    @Override
    public void toggleGamepad() {
        boolean show = gamepad.getVisibility() != View.VISIBLE;
        gamepad.setVisibility(show ? View.VISIBLE : View.GONE);
        preferences().edit().putBoolean(KEY_GAMEPAD, show).apply();
        screen.requestFocus();
    }

    private void startImport() {
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        intent.setType("*/*");
        intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true);
        startActivityForResult(intent, REQUEST_IMPORT);
    }

    private void startExport() {
        List<String> names = drive.fileNames();
        if (names.isEmpty()) {
            Toast.makeText(this, R.string.export_empty, Toast.LENGTH_SHORT).show();
            return;
        }
        String[] items = names.toArray(new String[0]);
        new AlertDialog.Builder(this)
                .setTitle(R.string.export_title)
                .setItems(items, (dialog, which) -> {
                    pendingExport = items[which];
                    Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT);
                    intent.addCategory(Intent.CATEGORY_OPENABLE);
                    intent.setType("application/octet-stream");
                    intent.putExtra(Intent.EXTRA_TITLE, pendingExport);
                    startActivityForResult(intent, REQUEST_EXPORT);
                })
                .show();
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (resultCode != RESULT_OK || data == null) {
            return;
        }
        if (requestCode == REQUEST_IMPORT) {
            List<Uri> uris = new ArrayList<>();
            ClipData clip = data.getClipData();
            if (clip != null) {
                for (int index = 0; index < clip.getItemCount(); index++) {
                    uris.add(clip.getItemAt(index).getUri());
                }
            } else if (data.getData() != null) {
                uris.add(data.getData());
            }
            importAll(uris);
        } else if (requestCode == REQUEST_EXPORT && pendingExport != null && data.getData() != null) {
            String name = pendingExport;
            pendingExport = null;
            try {
                drive.exportTo(name, data.getData());
                Toast.makeText(this, getString(R.string.exported, name), Toast.LENGTH_SHORT).show();
            } catch (IOException error) {
                Toast.makeText(this, getString(R.string.export_failed, name), Toast.LENGTH_LONG).show();
            }
        }
    }

    private void importAll(List<Uri> uris) {
        new Thread(() -> {
            for (Uri uri : uris) {
                String message;
                try {
                    message = getString(R.string.imported, drive.importUri(uri));
                } catch (IOException error) {
                    message = getString(R.string.import_failed, String.valueOf(uri.getLastPathSegment()));
                }
                String text = message;
                runOnUiThread(() -> Toast.makeText(this, text, Toast.LENGTH_SHORT).show());
            }
        }).start();
    }

    private void restartSession() {
        restarting = true;
        NativeBridge.stop();
        waitAndStart();
    }

    private void waitAndStart() {
        if (NativeBridge.isRunning()) {
            handler.postDelayed(this::waitAndStart, 50);
            return;
        }
        restarting = false;
        NativeBridge.start(drive.rootPathBytes());
        Toast.makeText(this, R.string.restarted, Toast.LENGTH_SHORT).show();
        screen.requestFocus();
    }

    @Override
    public void onSessionEnded() {
        if (!restarting && !isFinishing()) {
            handler.postDelayed(this::waitAndStart, 300);
        }
    }
}
