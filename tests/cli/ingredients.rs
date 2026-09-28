use super::*;

#[derive(serde::Deserialize)]
struct Ingredient {
    rules: Vec<IngredientRule>,
}

#[derive(serde::Deserialize)]
struct IngredientRule {
    #[serde(rename = "from")]
    source: String,
    targets: Vec<String>,
    to: String,
}

#[test]
fn every_library_ingredient_round_trips_through_the_cli() {
    let library = Path::new(env!("CARGO_MANIFEST_DIR")).join("ingredients/library");
    let mut ingredients: Vec<_> = fs::read_dir(&library)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "yaml")
        })
        .collect();
    ingredients.sort();
    assert!(!ingredients.is_empty(), "ingredient library is empty");

    for ingredient in ingredients {
        let name = ingredient.file_stem().unwrap().to_str().unwrap();
        let yaml = fs::read_to_string(&ingredient).unwrap();
        let rules: Ingredient = serde_yaml_ng::from_str(&yaml).unwrap();
        assert!(!rules.rules.is_empty(), "{name} has no rules");

        let dir = TempDir::new().unwrap();
        let home = dir.path();
        let mut expected = Vec::new();
        for rule in &rules.rules {
            let parent = rule
                .source
                .strip_prefix("~/")
                .or_else(|| (rule.source == "~").then_some(""))
                .expect("ingredient sources must be home-relative");
            assert!(
                !rule.targets.is_empty(),
                "{name} has a rule without targets"
            );
            for target in &rule.targets {
                assert!(
                    !target.contains(['*', '?', '[']),
                    "{name}: add a fixture strategy for glob target {target}"
                );
                let source = home.join(parent).join(target);
                let backup = home.join("backup").join(name).join(&rule.to).join(target);
                let contents = format!("{name}:{}", source.display());
                fs::create_dir_all(source.parent().unwrap()).unwrap();
                fs::write(&source, &contents).unwrap();
                expected.push((source, backup, contents));
            }
        }
        write(
            home,
            "config.yaml",
            &format!(
                "symlinks: reference\ndestination: backup\nrules: []\ninclude: [{{file: {}, wrap: {name}}}]\n",
                ingredient.display()
            ),
        );
        let run = |action| {
            Command::new(env!("CARGO_BIN_EXE_cockup"))
                .current_dir(home)
                .env("HOME", home)
                .env("NO_COLOR", "1")
                .args([action, "config.yaml", "-q"])
                .output()
                .unwrap()
        };
        let backup = run("backup");
        assert!(backup.status.success(), "{name}: {}", text(&backup));
        for (source, backup, contents) in &expected {
            assert_eq!(fs::read_to_string(backup).unwrap(), *contents, "{name}");
            fs::remove_file(source).unwrap();
        }
        let restore = run("restore");
        assert!(restore.status.success(), "{name}: {}", text(&restore));
        for (source, _, contents) in &expected {
            assert_eq!(fs::read_to_string(source).unwrap(), *contents, "{name}");
        }
    }
}
