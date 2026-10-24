//! Puts each feature of the classified layer into a class, and gives each class its colour.

use super::color;
use crate::diagram::{Attribute, Classify, Ramp, Scheme};
use crate::scene::Feature;
use crate::theme::Theme;
use crate::{Error, Result};

/// BAG joins mixed use with this separator: "woonfunctie, winkelfunctie".
const VALUE_SEPARATOR: &str = ", ";
const OTHER_LABEL: &str = "Other";
pub const UNKNOWN_LABEL: &str = "Unknown";

#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    pub label: String,
    /// "#RRGGBB".
    pub fill: String,
}

/// The classes of one classified layer, in legend order.
#[derive(Debug, Clone)]
pub struct Classes {
    pub title: String,
    pub note: Option<String>,
    pub classes: Vec<Class>,
    pub unknown_label: String,
    pub unknown_fill: String,
}

pub fn build(classify: &Classify, theme: &Theme) -> Result<Classes> {
    let classes = match &classify.scheme {
        Scheme::Sequential { breaks, labels } => {
            sequential_classes(breaks, labels, ramp_of(theme, classify.ramp), theme)?
        }
        Scheme::Categorical { categories } => {
            let labels: Vec<&str> = categories
                .iter()
                .map(|category| category.label.as_str())
                .collect();
            categorical_classes(&labels, theme)?
        }
    };

    Ok(Classes {
        title: classify.title.clone(),
        note: classify.note.clone(),
        classes,
        unknown_label: classify
            .unknown_label
            .clone()
            .unwrap_or_else(|| UNKNOWN_LABEL.to_string()),
        unknown_fill: color::resolve(&theme.palette, "@unknown")?,
    })
}

/// The theme's colours for `ramp`, its main ramp when it has no such one.
pub fn ramp_of(theme: &Theme, ramp: Ramp) -> &[String] {
    match ramp {
        Ramp::Water if !theme.water_ramp.is_empty() => &theme.water_ramp,
        _ => &theme.ramp,
    }
}

/// Index into `Classes::classes`, or `None` when the attribute is unknown.
pub fn class_of(feature: &Feature, classify: &Classify) -> Option<usize> {
    match &classify.scheme {
        Scheme::Sequential { breaks, .. } => {
            let value = numeric_value(feature, classify.attribute)?;
            Some(breaks.iter().filter(|&&limit| value >= limit).count())
        }
        Scheme::Categorical { categories } => {
            let value = text_value(feature, classify.attribute)?;
            let matched = value.split(VALUE_SEPARATOR).find_map(|part| {
                let part = part.trim();
                categories
                    .iter()
                    .position(|category| category.value == part)
            });
            // Unmatched values go to "Other", the class after the last category.
            Some(matched.unwrap_or(categories.len()))
        }
    }
}

/// n breaks make n + 1 classes, coloured evenly along the ramp.
fn sequential_classes(
    breaks: &[f64],
    labels: &[String],
    ramp: &[String],
    theme: &Theme,
) -> Result<Vec<Class>> {
    if labels.len() != breaks.len() + 1 {
        return Err(Error::Invalid(format!(
            "a sequential scheme with {} breaks needs {} labels, got {}",
            breaks.len(),
            breaks.len() + 1,
            labels.len()
        )));
    }
    let ramp = resolved_list(ramp, theme, "ramp")?;
    let fills = color::sample_ramp(&ramp, labels.len());

    Ok(labels
        .iter()
        .zip(fills)
        .map(|(label, fill)| Class {
            label: label.clone(),
            fill,
        })
        .collect())
}

/// One class per category, then "Other" in the last category colour. With
/// more categories than colours, the colours before the last repeat.
fn categorical_classes(labels: &[&str], theme: &Theme) -> Result<Vec<Class>> {
    let colours = resolved_list(&theme.categories, theme, "categories")?;
    let other_fill = colours[colours.len() - 1].clone();
    let cycle = (colours.len() - 1).max(1);
    let mut classes: Vec<Class> = labels
        .iter()
        .enumerate()
        .map(|(index, label)| Class {
            label: label.to_string(),
            fill: colours[index % cycle].clone(),
        })
        .collect();
    classes.push(Class {
        label: OTHER_LABEL.to_string(),
        fill: other_fill,
    });

    Ok(classes)
}

fn resolved_list(values: &[String], theme: &Theme, what: &str) -> Result<Vec<String>> {
    if values.is_empty() {
        return Err(Error::Invalid(format!(
            "theme {:?} has no {what} colours",
            theme.id
        )));
    }

    values
        .iter()
        .map(|value| color::resolve(&theme.palette, value))
        .collect()
}

fn numeric_value(feature: &Feature, attribute: Attribute) -> Option<f64> {
    match attribute {
        Attribute::YearBuilt => feature.attributes.year_built.map(f64::from),
        Attribute::HeightM => feature.attributes.height_m,
        Attribute::Value => feature.attributes.value,
        Attribute::Dwellings => feature.attributes.dwellings.map(f64::from),
        Attribute::FloorAreaM2 => feature.attributes.floor_area_m2,
        Attribute::UseClass => None,
    }
}

fn text_value(feature: &Feature, attribute: Attribute) -> Option<String> {
    match attribute {
        Attribute::UseClass => feature
            .attributes
            .use_class
            .clone()
            .filter(|value| !value.trim().is_empty()),
        Attribute::YearBuilt => feature.attributes.year_built.map(|year| year.to_string()),
        Attribute::HeightM | Attribute::Value | Attribute::Dwellings | Attribute::FloorAreaM2 => {
            numeric_value(feature, attribute).map(|value| value.to_string())
        }
    }
}
