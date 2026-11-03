use crate::bridge::bridge;
use crate::terminal::AndroidTerminal;
use dosbox::dos::filesystem::Drive;
use dosbox::machine::Machine;
use dosbox::shell::Shell;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

const BANNER: [&str; 5] = [
    "DOSBox-RS for Android - 8086 emulator with built-in MASM and LINK",
    "Drive C: is this app's storage. Use the menu to import .EXE/.COM/.ASM files.",
    "Run a program by typing its name, e.g.  MAZE   (HELP lists all commands)",
    "Keys: extra-keys bar for ESC/CTRL/arrows, gamepad for 2 players. CTRL+C stops.",
    "",
];

pub fn start(root: PathBuf) {
    let bridge = bridge();
    if bridge.running.swap(true, Ordering::SeqCst) {
        return;
    }
    bridge.stop.store(false, Ordering::SeqCst);
    if let Ok(mut keys) = bridge.keys.lock() {
        keys.clear();
    }
    let spawned = std::thread::Builder::new()
        .name("dosbox".to_string())
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let _ = panic::catch_unwind(AssertUnwindSafe(|| run(&root)));
            bridge.running.store(false, Ordering::SeqCst);
        });
    if spawned.is_err() {
        bridge.running.store(false, Ordering::SeqCst);
    }
}

pub fn stop() {
    let bridge = bridge();
    bridge.stop.store(true, Ordering::SeqCst);
    bridge.key_ready.notify_all();
}

fn first_run_commands(root: &Path) -> Vec<String> {
    let exists = |name: &str| root.join(name).is_file();
    if exists("MAZE.ASM") && !exists("MAZE.EXE") {
        vec!["masm MAZE.ASM;".to_string(), "link MAZE.OBJ;".to_string()]
    } else {
        Vec::new()
    }
}

fn run(root: &Path) {
    if std::fs::create_dir_all(root).is_err() {
        return;
    }
    let Ok(drive) = Drive::mount(root) else {
        return;
    };
    let commands = first_run_commands(root);
    let mut shell = Shell::new(Machine::new(Box::new(AndroidTerminal), drive));
    for line in BANNER {
        shell.println(line);
    }
    if shell.run_startup(&commands).is_ok() {
        let _ = shell.interactive();
    }
    shell.finish();
}
