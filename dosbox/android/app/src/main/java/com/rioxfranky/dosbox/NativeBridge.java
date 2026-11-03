package com.rioxfranky.dosbox;

final class NativeBridge {
    static final int COLUMNS = 80;
    static final int ROWS = 25;
    static final int CELL_BYTES = COLUMNS * ROWS * 2;
    static final int SCREEN_BYTES = CELL_BYTES + 4;

    static {
        System.loadLibrary("dosbox_android");
    }

    private NativeBridge() {
    }

    static native void start(byte[] rootPath);

    static native void stop();

    static native boolean isRunning();

    static native void sendKey(int scan, int ascii);

    static native void sendChar(int unicode);

    static native void sendCtrl(int letter);

    static native void sendBreak();

    static native void setSpeed(int speed);

    static native int readScreen(byte[] output);
}
