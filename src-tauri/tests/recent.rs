use graft_lib::recent;

#[test]
fn remembers_repositories_newest_first_and_forgets_missing_ones() {
    let root = std::env::temp_dir().join(format!("graft-recent-{}", std::process::id()));
    let (config, first, second) = (root.join("config"), root.join("first"), root.join("second"));
    for directory in [&first, &second] {
        std::fs::create_dir_all(directory).expect("create repository directory");
    }
    let path = |directory: &std::path::Path| directory.display().to_string();

    recent::remember(&config, &path(&first), "first").expect("remember first");
    recent::remember(&config, &path(&second), "second").expect("remember second");
    recent::remember(&config, &path(&first), "first").expect("reopen first");

    let names: Vec<String> = recent::list(&config).into_iter().map(|entry| entry.name).collect();
    assert_eq!(names, ["first", "second"], "reopening moves an entry to the front without duplicating it");

    std::fs::remove_dir_all(&second).expect("delete repository directory");
    let names: Vec<String> = recent::list(&config).into_iter().map(|entry| entry.name).collect();
    assert_eq!(names, ["first"]);
    std::fs::remove_dir_all(root).ok();
}
