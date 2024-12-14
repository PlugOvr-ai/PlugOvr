#[derive(Debug)]
#[allow(dead_code)]
pub struct Coords {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct FormFields {
    pub field_name: String,
    pub field_value: String,
    pub field_coords: Coords,
}

pub fn analyse_answer(ai_answer: &str) -> Option<Vec<FormFields>> {
    let mut form_fields: Vec<FormFields> = Vec::new();

    // Find JSON content with proper bracket matching
    if let Some(start) = ai_answer.find('[') {
        let mut bracket_count = 0;
        let mut end = start;

        for (i, c) in ai_answer[start..].char_indices() {
            match c {
                '[' => bracket_count += 1,
                ']' => {
                    bracket_count -= 1;
                    if bracket_count == 0 {
                        end = start + i;
                        break;
                    }
                }
                _ => {}
            }
        }

        if bracket_count == 0 {
            let json_str = &ai_answer[start..=end];

            // Try to parse the extracted JSON
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                // Check if it's an object
                if let Some(obj) = json.as_array() {
                    // Validate expected structure
                    for field_obj in obj {
                        if let Some(field_obj) = field_obj.as_object() {
                            // Check if required fields exist
                            let has_content = field_obj.contains_key("content");
                            let has_caption = field_obj.contains_key("caption");
                            let has_coords = field_obj.contains_key("coordinates");

                            if !has_content || !has_caption || !has_coords {
                                return None;
                            }

                            // Extract coordinates
                            let coords_str = field_obj["coordinates"].as_str().unwrap_or("[]");
                            // Remove brackets and split by comma
                            let coords: Vec<i32> = coords_str
                                .trim_matches(|p| p == '[' || p == ']')
                                .split(',')
                                .filter_map(|s| s.trim().parse().ok())
                                .collect();

                            if coords.len() == 4 {
                                let field_coords = Coords {
                                    x1: coords[0],
                                    y1: coords[1],
                                    x2: coords[2],
                                    y2: coords[3],
                                };

                                // Create and push FormFields
                                form_fields.push(FormFields {
                                    field_name: field_obj["caption"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string(),
                                    field_value: field_obj["content"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string(),
                                    field_coords,
                                });
                            }
                        } else {
                            return None;
                        }
                    }
                }
            }
        }
    }
    Some(form_fields)
}
