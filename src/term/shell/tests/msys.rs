use super::*;

#[test]
fn under_msys2_fish_reports_its_folder_as_cygpath_names_it_once_a_folder() {
    // cygpath's stand-in names a folder on drive C: and counts its runs.
    let fake = folder("cygpath");
    let runs = fake.join("runs");
    let names = format!(
        "#!/bin/sh\necho >> '{}'\nprintf 'C:%s\\n' \"$3\"\n",
        runs.display()
    );
    fs::write(fake.join("cygpath"), names).expect("cygpath");
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(fake.join("cygpath"), fs::Permissions::from_mode(0o755)).expect("a mode");
    let cygpath = fake.join("cygpath").display().to_string();
    let mut run = Run::with(
        &installed("fish"),
        &[],
        vec![("ELY_CYGPATH".into(), cygpath)],
    );
    let start = Path::new("/C:").join(run.start.strip_prefix("/").expect("a full path"));
    run.until("the folder", |run| run.folders.contains(&start));
    run.send("set -q ELY_CYGPATH; or echo ELY-GO''NE");
    run.until("the variable gone", |run| run.shows("ELY-GONE"));
    run.send("cd in*");
    run.until("inner folder", |run| {
        run.folders.contains(&start.join(INNER))
    });
    run.send("true");
    run.send("echo ELY-DO''NE");
    run.until("two more prompts", |run| run.shows("ELY-DONE"));
    run.finish();
    let count = fs::read_to_string(&runs).expect("the runs").lines().count();
    assert_eq!(count, 2, "one run for each folder, not each prompt");
    fs::remove_dir_all(&fake).expect("cleanup");
}
