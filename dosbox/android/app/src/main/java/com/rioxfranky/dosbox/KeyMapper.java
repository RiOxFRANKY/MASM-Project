package com.rioxfranky.dosbox;

import android.view.KeyEvent;

final class KeyMapper {
    static final int SCAN_ESC = 0x01;
    static final int SCAN_BACKSPACE = 0x0E;
    static final int SCAN_TAB = 0x0F;
    static final int SCAN_ENTER = 0x1C;
    static final int SCAN_UP = 0x48;
    static final int SCAN_DOWN = 0x50;
    static final int SCAN_LEFT = 0x4B;
    static final int SCAN_RIGHT = 0x4D;

    private KeyMapper() {
    }

    static void sendSpecial(int scan, int ascii) {
        NativeBridge.sendKey(scan, ascii);
    }

    static void sendEnter() {
        sendSpecial(SCAN_ENTER, 13);
    }

    static void sendEscape() {
        sendSpecial(SCAN_ESC, 27);
    }

    static void sendBackspace() {
        sendSpecial(SCAN_BACKSPACE, 8);
    }

    static void sendText(CharSequence text) {
        for (int index = 0; index < text.length(); index++) {
            sendCharacter(text.charAt(index), false);
        }
    }

    static void sendCharacter(int character, boolean ctrl) {
        if (character == '\n' || character == '\r') {
            sendEnter();
            return;
        }
        if (ctrl && Character.isLetter(character) && character < 0x80) {
            NativeBridge.sendCtrl(character);
            return;
        }
        NativeBridge.sendChar(character);
    }

    static boolean handle(KeyEvent event) {
        if (event.getAction() != KeyEvent.ACTION_DOWN) {
            return false;
        }
        int scan = special(event.getKeyCode());
        if (scan >= 0) {
            sendSpecial(scan, specialAscii(event.getKeyCode()));
            return true;
        }
        if (event.getKeyCode() == KeyEvent.KEYCODE_BACK) {
            return false;
        }
        int unicode = event.getUnicodeChar(event.getMetaState() & ~KeyEvent.META_CTRL_MASK);
        if (unicode <= 0) {
            return false;
        }
        sendCharacter(unicode, event.isCtrlPressed());
        return true;
    }

    private static int special(int keyCode) {
        switch (keyCode) {
            case KeyEvent.KEYCODE_DPAD_UP:
                return SCAN_UP;
            case KeyEvent.KEYCODE_DPAD_DOWN:
                return SCAN_DOWN;
            case KeyEvent.KEYCODE_DPAD_LEFT:
                return SCAN_LEFT;
            case KeyEvent.KEYCODE_DPAD_RIGHT:
                return SCAN_RIGHT;
            case KeyEvent.KEYCODE_ENTER:
            case KeyEvent.KEYCODE_NUMPAD_ENTER:
            case KeyEvent.KEYCODE_DPAD_CENTER:
                return SCAN_ENTER;
            case KeyEvent.KEYCODE_DEL:
                return SCAN_BACKSPACE;
            case KeyEvent.KEYCODE_TAB:
                return SCAN_TAB;
            case KeyEvent.KEYCODE_ESCAPE:
                return SCAN_ESC;
            case KeyEvent.KEYCODE_FORWARD_DEL:
                return 0x53;
            case KeyEvent.KEYCODE_INSERT:
                return 0x52;
            case KeyEvent.KEYCODE_MOVE_HOME:
                return 0x47;
            case KeyEvent.KEYCODE_MOVE_END:
                return 0x4F;
            case KeyEvent.KEYCODE_PAGE_UP:
                return 0x49;
            case KeyEvent.KEYCODE_PAGE_DOWN:
                return 0x51;
            default:
                if (keyCode >= KeyEvent.KEYCODE_F1 && keyCode <= KeyEvent.KEYCODE_F10) {
                    return 0x3B + (keyCode - KeyEvent.KEYCODE_F1);
                }
                if (keyCode == KeyEvent.KEYCODE_F11) {
                    return 0x85;
                }
                if (keyCode == KeyEvent.KEYCODE_F12) {
                    return 0x86;
                }
                return -1;
        }
    }

    private static int specialAscii(int keyCode) {
        switch (keyCode) {
            case KeyEvent.KEYCODE_ENTER:
            case KeyEvent.KEYCODE_NUMPAD_ENTER:
            case KeyEvent.KEYCODE_DPAD_CENTER:
                return 13;
            case KeyEvent.KEYCODE_DEL:
                return 8;
            case KeyEvent.KEYCODE_TAB:
                return 9;
            case KeyEvent.KEYCODE_ESCAPE:
                return 27;
            default:
                return 0;
        }
    }
}
