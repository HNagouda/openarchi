//! One ratios group laid out at a block's width: as bars, a pie with its
//! legend, or a plain table. Bars and pies are drawn as on the ratios export
//! page; the table is the report's own.

use std::fmt::Write;

use super::draw::Drawing;
use super::RatiosStyle;
use crate::analysis::Paint;
use crate::ratios::{group_chart, Group};

const HAIRLINE_MM: f64 = 0.15;
const TABLE_ROW_MM: f64 = 5.4;

pub fn draw(group: &Group, style: RatiosStyle, width: f64, paint: &Paint) -> Drawing {
    let mut body = String::new();
    let top = group_chart::header(&mut body, group, width, paint);
    let height = match style {
        RatiosStyle::Bars => group_chart::bars(&mut body, group, top, width, paint),
        RatiosStyle::Pie => group_chart::pie(&mut body, group, top, width, paint),
        RatiosStyle::Table => table(&mut body, group, top, width, paint),
    };

    Drawing {
        body,
        width,
        height,
    }
}

/// Each row's text over a hairline.
fn table(out: &mut String, group: &Group, top: f64, width: f64, paint: &Paint) -> f64 {
    let mut y = top;
    for row in &group.rows {
        let baseline = y + group_chart::TEXT_MM;
        group_chart::row_text(out, row, group.unit, 0.0, baseline, width, paint);
        y += TABLE_ROW_MM;
        let rule_y = y - group_chart::BAR_GAP_MM;
        let _ = writeln!(
            out,
            r#"<line x1="0" y1="{rule_y:.2}" x2="{width:.2}" y2="{rule_y:.2}" stroke="{}" stroke-width="{HAIRLINE_MM}"/>"#,
            paint.faint
        );
    }

    y
}
