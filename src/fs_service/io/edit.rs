use crate::{
    error::ServiceResult,
    fs_service::{
        FileSystemService,
        utils::{detect_line_ending, normalize_line_endings},
    },
    tools::EditOperation,
};
use rust_mcp_sdk::schema::RpcError;
use similar::TextDiff;
use std::path::Path;

impl FileSystemService {
    pub fn create_unified_diff(
        &self,
        original_content: &str,
        new_content: &str,
        filepath: Option<String>,
    ) -> String {
        // Ensure consistent line endings for diff
        let normalized_original = normalize_line_endings(original_content);
        let normalized_new = normalize_line_endings(new_content);

        // // Generate the diff using TextDiff
        let diff = TextDiff::from_lines(&normalized_original, &normalized_new);

        let file_name = filepath.unwrap_or("file".to_string());
        // Format the diff as a unified diff
        let patch = diff
            .unified_diff()
            .header(
                format!("{file_name}\toriginal").as_str(),
                format!("{file_name}\tmodified").as_str(),
            )
            .context_radius(4)
            .to_string();

        format!("Index: {}\n{}\n{}", file_name, "=".repeat(68), patch)
    }

    pub async fn apply_file_edits(
        &self,
        file_path: &Path,
        edits: Vec<EditOperation>,
        dry_run: Option<bool>,
        save_to: Option<&Path>,
    ) -> ServiceResult<String> {
        let allowed_directories = self.allowed_directories().await;
        let valid_path = self.validate_path(file_path, allowed_directories)?;

        // Read file content and normalize line endings
        let content_str = tokio::fs::read_to_string(&valid_path).await?;
        let original_line_ending = detect_line_ending(&content_str);
        let content_str = normalize_line_endings(&content_str);

        // Apply edits sequentially
        let mut modified_content = content_str.clone();

        for edit in edits {
            let normalized_old = normalize_line_endings(&edit.old_text);
            let normalized_new = normalize_line_endings(&edit.new_text);
            // If exact match exists, use it
            if modified_content.contains(&normalized_old) {
                modified_content = modified_content.replacen(&normalized_old, &normalized_new, 1);
                continue;
            }

            // Otherwise, try line-by-line matching with flexibility for whitespace
            let old_lines: Vec<String> = normalized_old
                .trim_end()
                .split('\n')
                .map(|s| s.to_string())
                .collect();

            let content_lines: Vec<String> = modified_content
                .trim_end()
                .split('\n')
                .map(|s| s.to_string())
                .collect();

            let mut match_found = false;

            // skip when the match is impossible:
            if old_lines.len() > content_lines.len() {
                let error_message = format!(
                    "Cannot apply edit: the original text spans more lines ({}) than the file content ({}).",
                    old_lines.len(),
                    content_lines.len()
                );

                return Err(RpcError::internal_error()
                    .with_message(error_message)
                    .into());
            }

            let max_start = content_lines.len().saturating_sub(old_lines.len());
            for i in 0..=max_start {
                let potential_match = &content_lines[i..i + old_lines.len()];

                // Compare lines with normalized whitespace
                let is_match = old_lines.iter().enumerate().all(|(j, old_line)| {
                    let content_line = &potential_match[j];
                    old_line.trim() == content_line.trim()
                });

                if is_match {
                    // Preserve original indentation of first line
                    let original_indent = content_lines[i]
                        .chars()
                        .take_while(|&c| c.is_whitespace())
                        .collect::<String>();

                    let new_lines: Vec<String> = normalized_new
                        .split('\n')
                        .enumerate()
                        .map(|(j, line)| {
                            // Keep indentation of the first line
                            if j == 0 {
                                return format!("{}{}", original_indent, line.trim_start());
                            }

                            // For subsequent lines, preserve relative indentation and original whitespace type
                            let old_indent = old_lines
                                .get(j)
                                .map(|line| {
                                    line.chars()
                                        .take_while(|&c| c.is_whitespace())
                                        .collect::<String>()
                                })
                                .unwrap_or_default();

                            let new_indent = line
                                .chars()
                                .take_while(|&c| c.is_whitespace())
                                .collect::<String>();

                            // Use the same whitespace character as original_indent (tabs or spaces)
                            let indent_char = if original_indent.contains('\t') {
                                "\t"
                            } else {
                                " "
                            };
                            let relative_indent = if new_indent.len() >= old_indent.len() {
                                new_indent.len() - old_indent.len()
                            } else {
                                0 // Don't reduce indentation below original
                            };
                            format!(
                                "{}{}{}",
                                &original_indent,
                                &indent_char.repeat(relative_indent),
                                line.trim_start()
                            )
                        })
                        .collect();

                    let mut content_lines = content_lines.clone();
                    content_lines.splice(i..i + old_lines.len(), new_lines);
                    modified_content = content_lines.join("\n");
                    match_found = true;
                    break;
                }
            }
            if !match_found {
                return Err(RpcError::internal_error()
                    .with_message(format!(
                        "Could not find exact match for edit:\n{}",
                        edit.old_text
                    ))
                    .into());
            }
        }

        let diff = self.create_unified_diff(
            &content_str,
            &modified_content,
            Some(valid_path.display().to_string()),
        );

        // Format diff with appropriate number of backticks
        let mut num_backticks = 3;
        while diff.contains(&"`".repeat(num_backticks)) {
            num_backticks += 1;
        }
        let formatted_diff = format!(
            "{}diff\n{}{}\n\n",
            "`".repeat(num_backticks),
            diff,
            "`".repeat(num_backticks)
        );

        let is_dry_run = dry_run.unwrap_or(false);

        if !is_dry_run {
            let target = save_to.unwrap_or(valid_path.as_path());
            let modified_content = modified_content.replace("\n", original_line_ending);
            tokio::fs::write(target, modified_content).await?;
        }

        Ok(formatted_diff)
    }
}
