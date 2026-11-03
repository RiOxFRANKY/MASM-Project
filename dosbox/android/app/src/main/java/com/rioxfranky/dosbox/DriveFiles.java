package com.rioxfranky.dosbox;

import android.content.ContentResolver;
import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.provider.OpenableColumns;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Locale;

final class DriveFiles {
    private static final String ASSET_FOLDER = "C";

    private final Context context;
    private final File root;

    DriveFiles(Context context) {
        this.context = context;
        File base = context.getExternalFilesDir(null);
        if (base == null) {
            base = context.getFilesDir();
        }
        root = new File(base, "C");
    }

    File root() {
        return root;
    }

    byte[] rootPathBytes() {
        return root.getAbsolutePath().getBytes(StandardCharsets.UTF_8);
    }

    void installStarterFiles() {
        if (!root.isDirectory() && !root.mkdirs()) {
            return;
        }
        try {
            String[] names = context.getAssets().list(ASSET_FOLDER);
            if (names == null) {
                return;
            }
            for (String name : names) {
                byte[] bundled = readAsset(ASSET_FOLDER + "/" + name);
                File target = new File(root, name);
                if (target.exists() && Arrays.equals(bundled, readFile(target))) {
                    continue;
                }
                try (OutputStream output = new FileOutputStream(target)) {
                    output.write(bundled);
                }
                removeBuildOutputs(name);
            }
        } catch (IOException ignored) {
        }
    }

    private void removeBuildOutputs(String sourceName) {
        int dot = sourceName.lastIndexOf('.');
        String base = dot >= 0 ? sourceName.substring(0, dot) : sourceName;
        for (String extension : new String[]{".OBJ", ".EXE", ".MAP"}) {
            File output = new File(root, base + extension);
            if (output.exists() && !output.delete()) {
                output.deleteOnExit();
            }
        }
    }

    private byte[] readAsset(String path) throws IOException {
        try (InputStream input = context.getAssets().open(path)) {
            return readAll(input);
        }
    }

    private static byte[] readFile(File file) {
        try (InputStream input = new FileInputStream(file)) {
            return readAll(input);
        } catch (IOException error) {
            return new byte[0];
        }
    }

    private static byte[] readAll(InputStream input) throws IOException {
        java.io.ByteArrayOutputStream buffer = new java.io.ByteArrayOutputStream();
        copy(input, buffer);
        return buffer.toByteArray();
    }

    static String dosName(String original) {
        String name = original == null ? "FILE" : original;
        int slash = Math.max(name.lastIndexOf('/'), name.lastIndexOf('\\'));
        name = name.substring(slash + 1).toUpperCase(Locale.ROOT);
        int dot = name.lastIndexOf('.');
        String base = dot >= 0 ? name.substring(0, dot) : name;
        String extension = dot >= 0 ? name.substring(dot + 1) : "";
        base = clean(base, 8);
        extension = clean(extension, 3);
        if (base.isEmpty()) {
            base = "FILE";
        }
        return extension.isEmpty() ? base : base + "." + extension;
    }

    private static String clean(String text, int limit) {
        StringBuilder builder = new StringBuilder();
        for (char character : text.toCharArray()) {
            if (builder.length() >= limit) {
                break;
            }
            if ((character >= 'A' && character <= 'Z') || (character >= '0' && character <= '9') || "_-!#$%&@^~".indexOf(character) >= 0) {
                builder.append(character);
            } else if (character == ' ') {
                builder.append('_');
            }
        }
        return builder.toString();
    }

    String importUri(Uri uri) throws IOException {
        ContentResolver resolver = context.getContentResolver();
        String display = null;
        try (Cursor cursor = resolver.query(uri, new String[]{OpenableColumns.DISPLAY_NAME}, null, null, null)) {
            if (cursor != null && cursor.moveToFirst()) {
                display = cursor.getString(0);
            }
        }
        String name = dosName(display);
        if (!root.isDirectory() && !root.mkdirs()) {
            throw new IOException("drive C: is not available");
        }
        try (InputStream input = resolver.openInputStream(uri);
             OutputStream output = new FileOutputStream(new File(root, name))) {
            if (input == null) {
                throw new IOException("cannot open " + display);
            }
            copy(input, output);
        }
        return name;
    }

    List<String> fileNames() {
        List<String> names = new ArrayList<>();
        File[] files = root.listFiles();
        if (files == null) {
            return names;
        }
        Arrays.sort(files);
        for (File file : files) {
            if (file.isFile()) {
                names.add(file.getName());
            }
        }
        return names;
    }

    void exportTo(String name, Uri target) throws IOException {
        try (InputStream input = new FileInputStream(new File(root, name));
             OutputStream output = context.getContentResolver().openOutputStream(target)) {
            if (output == null) {
                throw new IOException("cannot write " + target);
            }
            copy(input, output);
        }
    }

    private static void copy(InputStream input, OutputStream output) throws IOException {
        byte[] buffer = new byte[16 * 1024];
        int count;
        while ((count = input.read(buffer)) > 0) {
            output.write(buffer, 0, count);
        }
    }
}
