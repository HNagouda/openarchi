use super::*;
use crate::catalog::{builtin_themes, theme_problems};

const FIVE: &str = "f79f79, f7d08a, e3f09b, 87b6a7, 5b5941";

fn draft(text: &str) -> PaletteDraft {
    PaletteDraft {
        name: "Five".to_string(),
        swatches: parse(text).unwrap(),
        roles: RoleChoice::default(),
        typography: None,
        based_on: None,
    }
}

fn role<'a>(check: &'a PaletteCheck, name: &str) -> &'a RoleColour {
    check
        .roles
        .iter()
        .find(|colour| colour.role == name)
        .unwrap()
}

#[test]
fn five_colours_fill_the_roles_by_the_rule() {
    let check = check(&draft(FIVE), "five").unwrap();
    // Mindaro is the lightest, but colourful, so the paper is a pale tint of it.
    assert_eq!(role(&check, "paper").swatch, Some(2));
    assert!(role(&check, "paper").reason.contains("softened"));
    assert_eq!(role(&check, "ink").hex, "#5B5941");
    // No swatch left stands out from the pale paper by 3:1, so buildings take the ink.
    assert_eq!(role(&check, "figure").hex, "#5B5941");
    assert_eq!(role(&check, "accent").hex, "#F79F79");
    // Cambridge blue is the only cool swatch, so water and green share it, water fuller.
    assert_eq!(role(&check, "water").swatch, Some(3));
    assert_eq!(role(&check, "green").swatch, Some(3));
    assert!(lch(&role(&check, "water").hex).lightness < lch(&role(&check, "green").hex).lightness);
}

#[test]
fn a_warm_palette_gets_grey_hatched_water() {
    let check = check(&draft("ffffff, d62828, f77f00, 3a1f04"), "warm").unwrap();
    assert_eq!(role(&check, "water").swatch, None);
    let water = &check.theme.layers[&crate::scene::Layer::Water];
    assert!(water.pattern.is_some());
}

#[test]
fn a_generated_theme_passes_the_catalog_checks() {
    for text in [
        FIVE,
        "000000, ffffff",
        "1c3d6e, f2f2f2, e07a5f, 3d405b, 81b29a, f4f1de",
    ] {
        let check = check(&draft(text), "mine").unwrap();
        assert_eq!(theme_problems(&check.theme), Vec::<String>::new(), "{text}");
        assert_eq!(check.theme.categories.len(), 12);
        assert_eq!(check.theme.ramp.len(), 6);
    }
}

#[test]
fn every_layer_of_the_template_is_drawn_and_no_raw_colour_survives() {
    let template = builtin_themes()
        .into_iter()
        .find(|theme| theme.id == "paper")
        .unwrap();
    let theme = check(&draft(FIVE), "five").unwrap().theme;
    assert_eq!(
        theme.layers.keys().collect::<Vec<_>>(),
        template.layers.keys().collect::<Vec<_>>()
    );
    for (layer, style) in &theme.layers {
        let old = &template.layers[layer];
        for (new, old) in [(&style.fill, &old.fill), (&style.stroke, &old.stroke)] {
            if let (Some(new), Some(old)) = (new, old) {
                if old.starts_with('#') {
                    assert_ne!(new, old, "{layer:?} kept the template's colour");
                }
            }
        }
    }
}

#[test]
fn a_pinned_role_takes_its_swatch_exactly_and_the_rule_fills_around_it() {
    let mut pinned = draft(FIVE);
    pinned.roles.paper = Some(1);
    let check = check(&pinned, "five").unwrap();
    assert_eq!(role(&check, "paper").hex, "#F7D08A");
    assert!(role(&check, "paper").is_pinned);
    // Jasmine is taken, so the accent stays the most colourful of the rest.
    assert_eq!(role(&check, "accent").hex, "#F79F79");
}

#[test]
fn a_pin_past_the_last_swatch_is_refused() {
    let mut pinned = draft(FIVE);
    pinned.roles.ink = Some(9);
    let error = check(&pinned, "five").unwrap_err().to_string();
    assert!(error.contains("ink is pinned to swatch 9"), "{error}");
}

#[test]
fn low_contrast_ink_is_named_with_its_wcag_ratio() {
    let check = check(&draft("f7d08a, f79f79"), "pale").unwrap();
    let warning = check.warnings.first().expect("a contrast warning");
    assert!(warning.contains("4.5:1"), "{warning}");
    assert!(warning.contains("#F79F79"), "{warning}");
}

#[test]
fn black_and_white_reads_without_warnings() {
    assert!(check(&draft("000000, ffffff, d62828"), "bw")
        .unwrap()
        .warnings
        .is_empty());
}

#[test]
fn a_duplicated_builtin_keeps_its_main_colours() {
    for theme in builtin_themes() {
        let copy = draft_from(&theme, "Copy");
        assert!(copy.swatches.len() <= MAX_SWATCHES, "{}", theme.id);
        let made = check(&copy, "copy").unwrap().theme;
        assert_eq!(made.palette.paper, theme.palette.paper, "{}", theme.id);
        assert_eq!(made.palette.ink, theme.palette.ink, "{}", theme.id);
        assert_eq!(made.palette.figure, theme.palette.figure, "{}", theme.id);
        assert_eq!(made.palette.accent, theme.palette.accent, "{}", theme.id);
        assert_eq!(made.typography, theme.typography, "{}", theme.id);
    }
}

#[test]
fn a_saved_palette_opens_again_with_its_swatches_and_pins() {
    let mut pinned = draft(FIVE);
    pinned.roles.accent = Some(3);
    let theme = check(&pinned, "five").unwrap().theme;
    let again = draft_from(&theme, "Five");
    assert_eq!(again.swatches, pinned.swatches);
    assert_eq!(again.roles, pinned.roles);
}

#[test]
fn an_unchanged_copy_keeps_the_builtin_look_and_a_changed_one_is_mixed() {
    let paper = builtin_themes()
        .into_iter()
        .find(|theme| theme.id == "paper")
        .unwrap();
    let copy = draft_from(&paper, "Paper, mine");
    let kept = check_copy(&copy, "paper-mine", &paper).unwrap().theme;
    assert_eq!(kept.layers, paper.layers);
    assert_eq!(kept.ramp, paper.ramp);
    assert_eq!(kept.name, "Paper, mine");

    let mut changed = copy.clone();
    changed.swatches[0].hex = "#FFFFFF".to_string();
    let mixed = check_copy(&changed, "paper-mine", &paper).unwrap().theme;
    assert_eq!(mixed.palette.paper, "#FFFFFF");
    assert_ne!(mixed.layers, paper.layers);
}

#[test]
fn a_saved_copy_opened_again_unchanged_keeps_the_builtin_look() {
    let paper = builtin_themes()
        .into_iter()
        .find(|theme| theme.id == "paper")
        .unwrap();
    let saved = check_copy(&draft_from(&paper, "Paper, mine"), "paper-mine", &paper)
        .unwrap()
        .theme;
    let reopened = draft_from(&saved, "Paper, mine");
    let again = check_copy(&reopened, "paper-mine", &saved).unwrap().theme;
    assert_eq!(again.layers, paper.layers);
    assert_eq!(again.ramp, paper.ramp);
}
