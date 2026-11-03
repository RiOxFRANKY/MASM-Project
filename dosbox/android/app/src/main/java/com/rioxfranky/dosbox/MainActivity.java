package com.rioxfranky.dosbox;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.ClipData;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.Menu;
import android.widget.LinearLayout;
import android.widget.PopupMenu;
import android.widget.Toast;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;

public final class MainActivity extends Activity implements ScreenView.SessionListener {
    private static final int REQUEST_IMPORT = 1;
    private static final int REQUEST_EXPORT = 2;
    private static final int MENU_IMPORT = 1;
    private static final int MENU_EXPORT = 2;
    private static final int MENU_RESTART = 3;
    private static final int[] SPEEDS = {1, 2, 4};

    private final Handler handler = new Handler(Looper.getMainLooper());
    private DriveFiles drive;
    private ScreenView screen;
    private KeyButton fastForward;
    private int speedIndex;
    private String pendingExport;
    private boolean restarting;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        drive = new DriveFiles(this);
        drive.installStarterFiles();
        float density = getResources().getDisplayMetrics().density;

        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setBackgroundColor(0xFF000000);

        screen = new ScreenView(this);
        screen.setSessionListener(this);
        root.addView(screen, new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, 0, 1f));
        ControllerView controller = buildController();
        int gap = Math.round(8 * density);
        controller.setPadding(0, gap, 0, gap);
        root.addView(controller, new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT));
        setContentView(root);

        NativeBridge.setSpeed(SPEEDS[speedIndex]);
        NativeBridge.start(drive.rootPathBytes());
        screen.requestFocus();
    }

    private ControllerView buildController() {
        ControllerView controller = new ControllerView(this);
        KeyButton menu = new KeyButton(this, "⋮", KeyButton.Shape.PILL, false, () -> { });
        menu.setOnClickListener(view -> showMenu(menu));
        fastForward = new KeyButton(this, "►►", KeyButton.Shape.PILL, false, this::cycleSpeed);
        controller.addRect(menu, 0.86f, 0.05f, 0.986f, 0.145f);
        controller.addRect(new KeyButton(this, "LEFT", KeyButton.Shape.PILL, false,
                () -> NativeBridge.sendKey(0x02, '1')), 0.012f, 0.323f, 0.213f, 0.418f);
        controller.addRect(fastForward, 0.232f, 0.323f, 0.436f, 0.418f);
        controller.addCircle(new DpadView(this), 0.278f, 0.718f, 0.267f);
        controller.addRect(new KeyButton(this, "SELECT", KeyButton.Shape.PILL, false,
                screen::toggleKeyboard), 0.564f, 0.55f, 0.766f, 0.646f);
        controller.addRect(new KeyButton(this, "START", KeyButton.Shape.PILL, false,
                () -> NativeBridge.sendKey(0x04, '3')), 0.784f, 0.55f, 0.986f, 0.646f);
        controller.addRect(new KeyButton(this, "RIGHT", KeyButton.Shape.PILL, false,
                () -> NativeBridge.sendKey(0x03, '2')), 0.784f, 0.665f, 0.986f, 0.761f);
        controller.addCircle(new KeyButton(this, "B", KeyButton.Shape.CIRCLE, false,
                KeyMapper::sendEscape), 0.666f, 0.885f, 0.093f);
        controller.addCircle(new KeyButton(this, "A", KeyButton.Shape.CIRCLE, false,
                KeyMapper::sendEnter), 0.889f, 0.885f, 0.093f);
        return controller;
    }

    private void cycleSpeed() {
        speedIndex = (speedIndex + 1) % SPEEDS.length;
        int speed = SPEEDS[speedIndex];
        NativeBridge.setSpeed(speed);
        fastForward.setText(speed == 1 ? "►►" : "►► " + speed + "x");
    }

    private void showMenu(KeyButton anchor) {
        PopupMenu popup = new PopupMenu(this, anchor);
        popup.getMenu().add(Menu.NONE, MENU_IMPORT, 1, R.string.menu_import);
        popup.getMenu().add(Menu.NONE, MENU_EXPORT, 2, R.string.menu_export);
        popup.getMenu().add(Menu.NONE, MENU_RESTART, 3, R.string.menu_restart);
        popup.setOnMenuItemClickListener(item -> {
            switch (item.getItemId()) {
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
                    return false;
            }
        });
        popup.show();
    }

    @Override
    protected void onDestroy() {
        if (isFinishing()) {
            NativeBridge.stop();
        }
        super.onDestroy();
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
