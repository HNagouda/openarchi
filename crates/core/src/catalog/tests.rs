use super::*;

#[test]
fn every_builtin_theme_is_valid() {
    for theme in builtin_themes() {
        assert_eq!(
            theme_problems(&theme),
            Vec::<String>::new(),
            "theme {}",
            theme.id
        );
    }
}

#[test]
fn builtin_ids_match_their_file_names() {
    for ((name, _), theme) in BUILTIN_THEMES.iter().zip(builtin_themes()) {
        assert_eq!(*name, theme.id);
    }
    for ((name, _), diagram) in BUILTIN_DIAGRAMS.iter().zip(builtin_diagrams()) {
        assert_eq!(*name, diagram.id);
    }
}

#[test]
fn diagrams_of_one_group_sit_together() {
    let groups: Vec<String> = builtin_diagrams().into_iter().map(|d| d.group).collect();
    let mut seen: Vec<&String> = Vec::new();
    for (index, group) in groups.iter().enumerate() {
        let is_new = index == 0 || groups[index - 1] != *group;
        if is_new {
            assert!(!seen.contains(&group), "group {group:?} is split up");
            seen.push(group);
        }
    }
}

#[test]
fn a_diagram_with_variants_takes_its_layers_only_from_them() {
    for diagram in builtin_diagrams() {
        if diagram.variants.is_empty() {
            assert!(!diagram.layers.is_empty(), "{} draws nothing", diagram.id);
            continue;
        }
        assert!(diagram.layers.is_empty(), "{} has layers", diagram.id);
        assert!(diagram.emphasis.is_none(), "{} has an emphasis", diagram.id);
        assert!(diagram.classify.is_none(), "{} classifies", diagram.id);

        let mut ids: Vec<&str> = diagram.variants.iter().map(|v| v.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(
            ids.len(),
            diagram.variants.len(),
            "{} repeats a variant",
            diagram.id
        );
        for variant in &diagram.variants {
            assert!(!variant.layers.is_empty(), "{}:{}", diagram.id, variant.id);
            assert!(!variant.id.contains(':'), "{}:{}", diagram.id, variant.id);
        }
    }
}
