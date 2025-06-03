use std::{fs, thread};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;
use chrono::prelude::*;

// Verschiebt den bestehenden Backup-Ordner auf einen Zeitstempel-Ordner
fn store_backup() -> std::io::Result<()> {
    let backup_config = Path::new("/home/lennilid/backup/config");
    if backup_config.exists() {
        let date = Local::now();
        let new_path = format!(
            "/home/lennilid/backup/config-{}",
            date.format("%Y-%m-%d_%H-%M")
        );
        fs::rename(&backup_config, new_path)?;
    }
    Ok(())
}

// Stellt sicher, dass `backup_dir` ein Git-Repo mit Remote `remote_url` ist.
fn ensure_git_repo(backup_dir: &str, remote_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let git_folder = Path::new(backup_dir).join(".git");
    if !git_folder.exists() {
        println!("→ Initialisiere Git-Repository in `{}` …", backup_dir);
        Command::new("git")
            .args(&["init", "--initial-branch=main"])
            .current_dir(backup_dir)
            .status()?
            .success()
            .then(|| ())
            .ok_or("Git init ist fehlgeschlagen")?;
    }

    // Stelle sicher, dass wir auf dem Branch "main" sind
    Command::new("git")
        .args(&["checkout", "-B", "main"])
        .current_dir(backup_dir)
        .status()?
        .success()
        .then(|| ())
        .ok_or("Git checkout -B main ist fehlgeschlagen")?;

    // Liste der vorhandenen Remotes auslesen
    let existing_remotes = Command::new("git")
        .args(&["remote"])
        .current_dir(backup_dir)
        .output()?;
    let out = String::from_utf8_lossy(&existing_remotes.stdout);

    if !out.lines().any(|line| line.trim() == "origin") {
        println!("→ Setze Remote origin auf `{}` …", remote_url);
        Command::new("git")
            .args(&["remote", "add", "origin", remote_url])
            .current_dir(backup_dir)
            .status()?
            .success()
            .then(|| ())
            .ok_or("Git remote add origin ist fehlgeschlagen")?;
    } else {
        // Wenn origin existiert, URL ggf. aktualisieren
        Command::new("git")
            .args(&["remote", "set-url", "origin", remote_url])
            .current_dir(backup_dir)
            .status()?
            .success()
            .then(|| ())
            .ok_or("Git remote set-url origin ist fehlgeschlagen")?;
    }

    Ok(())
}

// Fügt alle Änderungen hinzu, committet und pusht ins Repo (erzwingt force-push)
fn git_push(backup_dir: &str, remote_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    ensure_git_repo(backup_dir, remote_url)?;

    // git add .
    Command::new("git")
        .args(&["add", "."])
        .current_dir(backup_dir)
        .status()?
        .success()
        .then(|| ())
        .ok_or("Git add ist fehlgeschlagen")?;

    // git commit -m "Backup am <Datum>"
    let msg = format!("Backup am {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    let commit = Command::new("git")
        .args(&["commit", "-m", &msg])
        .current_dir(backup_dir)
        .status()?;
    // Exit-Code 1 = nichts zu committen → ignorieren
    if !commit.success() && commit.code() != Some(1) {
        return Err("Git commit ist fehlgeschlagen".into());
    }

    // Push per force, um remote komplett zu überschreiben
    let push = Command::new("git")
        .args(&["push", "--force", "origin", "main"])
        .current_dir(backup_dir)
        .status()?;
    if !push.success() {
        return Err("Git push --force ist fehlgeschlagen".into());
    }

    println!("✅ Backup erfolgreich gepusht an `{}`", remote_url);
    Ok(())
}

fn backup() -> Result<(), Box<dyn std::error::Error>> {
    // === 1. Backup-Ordner und Quelle definieren ===
    let source_root = Path::new("/home/lennilid/.config/");
    let backup_root = Path::new("/home/lennilid/backup/config/");
    let backup_dir_str = backup_root.to_str().unwrap(); // Für git_push-Aufruf

    // === 2. Blacklist definieren (Beispiel: "discord" und "cache") ===
    let blacklist = ["discord", "cache"];

    // Sicherstellen, dass das Ziel-Verzeichnis existiert
    fs::create_dir_all(&backup_root)?;

    // === 3. Dateien rekursiv kopieren, Blacklist ausfiltern ===
    for entry in WalkDir::new(&source_root) {
        let entry = entry?;
        let src_path = entry.path();

        if src_path.is_file() {
            let rel_path = src_path.strip_prefix(&source_root)
                .expect("strip_prefix sollte nie fehlschlagen");
            let rel_str = rel_path.to_string_lossy().to_lowercase();

            // Blacklist-Check: Term kommt irgendwo im relativen Pfad vor?
            if blacklist.iter().any(|term| rel_str.contains(term)) {
                continue;
            }

            let dest_path: PathBuf = backup_root.join(rel_path);
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src_path, &dest_path)?;
            println!("copied {:?} -> {:?}", src_path, dest_path);
        }
    }

    println!("Backup abgeschlossen!");

    let remote_url = "git@github.com:LenniLID/backup-config.git";
    git_push(backup_dir_str, remote_url)?;
    Ok(())
}

fn main() {
    loop {
        // 1. Vor dem Kopieren den alten Backup-Ordner umbenennen
        if let Err(e) = store_backup() {
            eprintln!("Fehler beim Umbenennen des alten Backups: {}", e);
        }

        // 2. Neues Backup erstellen und pushen
        if let Err(e) = backup() {
            eprintln!("Fehler beim Backup: {}", e);
        }

        // 3. 5 Stunden warten, bevor die nächste Runde startet
        thread::sleep(std::time::Duration::from_secs(60 * 60 * 5));
    }
}
