package com.rioxfranky.dosbox;

import android.view.KeyEvent;
import android.view.View;
import android.view.inputmethod.BaseInputConnection;

final class DosInputConnection extends BaseInputConnection {
    private String composing = "";

    DosInputConnection(View view) {
        super(view, false);
    }

    private void replaceComposing(String text) {
        int common = 0;
        int limit = Math.min(composing.length(), text.length());
        while (common < limit && composing.charAt(common) == text.charAt(common)) {
            common++;
        }
        for (int index = common; index < composing.length(); index++) {
            KeyMapper.sendBackspace();
        }
        KeyMapper.sendText(text.substring(common));
        composing = text;
    }

    @Override
    public boolean commitText(CharSequence text, int newCursorPosition) {
        replaceComposing(text.toString());
        composing = "";
        return true;
    }

    @Override
    public boolean setComposingText(CharSequence text, int newCursorPosition) {
        replaceComposing(text.toString());
        return true;
    }

    @Override
    public boolean finishComposingText() {
        composing = "";
        return true;
    }

    @Override
    public boolean deleteSurroundingText(int beforeLength, int afterLength) {
        int count = Math.max(beforeLength, 1);
        for (int index = 0; index < count; index++) {
            KeyMapper.sendBackspace();
        }
        return true;
    }

    @Override
    public boolean sendKeyEvent(KeyEvent event) {
        KeyMapper.handle(event);
        return true;
    }
}
