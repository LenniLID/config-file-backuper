use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

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
        // Wenn origin existiert, aktualisiere die URL (falls sich remote_url geändert hat)
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

// Fügt alle Änderungen hinzu, committet und pusht ins Repo
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
    let msg = format!("Backup am {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
    let commit = Command::new("git")
        .args(&["commit", "-m", &msg])
        .current_dir(backup_dir)
        .status()?;
    // Exit-Code 1 = nichts zu committen → ignorieren
    if !commit.success() && commit.code() != Some(1) {
        return Err("Git commit ist fehlgeschlagen".into());
    }

    // 1) Hole Remote-Änderungen und rebase
    let pull = Command::new("git")
        .args(&["pull", "--rebase", "origin", "main"])
        .current_dir(backup_dir)
        .status()?;
    // Pull schlägt nur dann wirklich fehl, wenn z.B. kein Netzwerk oder Konflikte da sind.
    // Wir lassen hier einen Non-Zero-Code durchgehen, solange es kein schwerwiegender Fehler war.
    if !pull.success() {
        println!("⚠️  Warnung: `git pull --rebase` ist fehlgeschlagen. Prüfe manuell, ob Konflikte vorliegen.");
    }

    // 2) git push --set-upstream origin main (nur beim ersten Mal nötig, später genügt 'git push')
    let push = Command::new("git")
        .args(&["push", "--set-upstream", "origin", "main"])
        .current_dir(backup_dir)
        .status()?;
    if !push.success() {
        return Err("Git push ist fehlgeschlagen. Überprüfe SSH-Key oder Remote-URL.".into());
    }

    println!("✅ Backup erfolgreich gepusht an `{}`", remote_url);
    Ok(())
}



fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // === 4. Git-Push ins separate Repo ===
    //    Hier kannst du zwischen SSH-URL oder HTTPS-URL wählen:
    //
    // SSH-Variante:
    // let remote_url = "git@github.com:DEIN_USERNAME/DEIN_BACKUP_REPO.git";
    //
    // HTTPS-Variante (falls kein SSH-Key):
    // let remote_url = "https://github.com/DEIN_USERNAME/DEIN_BACKUP_REPO.git";

    let remote_url = "git@github.com:LenniLID/backup-config.git";
    git_push(backup_dir_str, remote_url)?;

    Ok(())
}
