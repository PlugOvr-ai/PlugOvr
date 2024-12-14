use similar::{ChangeTag, TextDiff};

pub fn display_diff(ui: &mut egui::Ui, old_text: &str, new_text: &str) {
    let diff = TextDiff::from_chars(old_text, new_text);

    let mut job = egui::text::LayoutJob::default();
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => job.append(
                &change
                    .to_string()
                    .chars()
                    .next()
                    .unwrap_or_default()
                    .to_string(),
                0.0,
                egui::TextFormat {
                    color: ui.style().visuals.text_color(),
                    background: egui::Color32::from_rgb(255, 0, 0),
                    font_id: ui
                        .style()
                        .text_styles
                        .get(&egui::TextStyle::Body)
                        .expect("Body style not found")
                        .clone(),
                    ..Default::default()
                },
            ),
            ChangeTag::Insert => job.append(
                &change
                    .to_string()
                    .chars()
                    .next()
                    .unwrap_or_default()
                    .to_string(),
                0.0,
                egui::TextFormat {
                    color: ui.style().visuals.text_color(),
                    background: egui::Color32::from_rgb(0, 255, 0),
                    font_id: ui
                        .style()
                        .text_styles
                        .get(&egui::TextStyle::Body)
                        .expect("Body style not found")
                        .clone(),
                    ..Default::default()
                },
            ),
            ChangeTag::Equal => job.append(
                &change
                    .to_string()
                    .chars()
                    .next()
                    .unwrap_or_default()
                    .to_string(),
                0.0,
                egui::TextFormat {
                    color: ui.style().visuals.text_color(),
                    font_id: ui
                        .style()
                        .text_styles
                        .get(&egui::TextStyle::Body)
                        .expect("Body style not found")
                        .clone(),
                    ..Default::default()
                },
            ),
        };
    }
    ui.label(job);
}
