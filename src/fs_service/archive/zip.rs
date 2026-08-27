use crate::{
    error::ServiceResult,
    fs_service::{FileSystemService, walk_dir},
};
use cap_std::fs::Dir;
use glob_match::glob_match;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::CompressionMethod;
use zip::write::ZipWriter;

fn format_bytes_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    let units = [(TB, "TB"), (GB, "GB"), (MB, "MB"), (KB, "KB")];

    for (threshold, unit) in units {
        if bytes >= threshold {
            return format!("{:.2} {}", bytes as f64 / threshold as f64, unit);
        }
    }
    format!("{bytes} bytes")
}

impl FileSystemService {
    pub async fn zip_directory(
        &self,
        input_dir: String,
        pattern: String,
        target_zip_file: String,
    ) -> ServiceResult<String> {
        let resolved_input = self.resolve(Path::new(&input_dir)).await?;
        let resolved_target = self.resolve(Path::new(&target_zip_file)).await?;

        if resolved_target.dir.exists(&resolved_target.rel) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("'{target_zip_file}' already exists!"),
            )
            .into());
        }

        let updated_pattern = if pattern.contains('*') {
            pattern.to_lowercase()
        } else {
            format!("*{}*", &pattern.to_lowercase())
        };
        let glob_pattern = updated_pattern;

        // Confined recursive traversal.
        let mut all_entries = Vec::new();
        walk_dir(
            &resolved_input.dir,
            &resolved_input.rel,
            &resolved_input.display,
            &mut all_entries,
        )?;

        let mut entries: Vec<PathBuf> = Vec::new();
        for entry in all_entries {
            if entry.is_dir {
                continue;
            }
            let rel_from_input = entry
                .rel
                .strip_prefix(&resolved_input.rel)
                .unwrap_or(&entry.rel);
            if glob_match(&glob_pattern, rel_from_input.to_str().unwrap_or("")) {
                entries.push(entry.rel);
            }
        }

        let dir = resolved_input.dir.try_clone()?;
        let input_rel = resolved_input.rel.clone();
        let target_dir = resolved_target.dir.try_clone()?;
        let target_rel = resolved_target.rel.clone();

        let zip_file_size = tokio::task::spawn_blocking(move || {
            let file = target_dir.create(&target_rel)?;
            let mut zip_writer = ZipWriter::new(file);
            let options: zip::write::FileOptions<()> =
                zip::write::FileOptions::default().compression_method(CompressionMethod::Deflated);

            for entry_rel in &entries {
                let rel_from_input = entry_rel
                    .strip_prefix(&input_rel)
                    .map_err(std::io::Error::other)?;
                let entry_str = rel_from_input
                    .to_str()
                    .ok_or_else(|| std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Invalid UTF-8 in file name",
                    ))?;

                let mut input_file = dir.open(entry_rel)?;
                let mut buffer = Vec::new();
                input_file.read_to_end(&mut buffer)?;

                zip_writer.start_file(entry_str, options)?;
                zip_writer.write_all(&buffer)?;
                zip_writer.flush()?;
            }

            zip_writer.finish()?;
            let metadata = target_dir.metadata(&target_rel)?;
            Ok::<u64, std::io::Error>(metadata.len())
        })
        .await
        .map_err(std::io::Error::other)??;

        let result_message = format!(
            "Successfully compressed '{}' directory into '{}' ({}).",
            input_dir,
            resolved_target.display.display(),
            format_bytes_size(zip_file_size)
        );
        Ok(result_message)
    }

    pub async fn zip_files(
        &self,
        input_files: Vec<String>,
        target_zip_file: String,
    ) -> ServiceResult<String> {
        let file_count = input_files.len();

        if file_count == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "No file(s) to zip. The input files array is empty.",
            )
            .into());
        }

        let resolved_target = self.resolve(Path::new(&target_zip_file)).await?;

        if resolved_target.dir.exists(&resolved_target.rel) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("'{target_zip_file}' already exists!"),
            )
            .into());
        }

        let mut sources: Vec<(Dir, PathBuf, String)> = Vec::with_capacity(file_count);
        for path in &input_files {
            let resolved = self.resolve(Path::new(path)).await?;
            let filename = resolved
                .rel
                .file_name()
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path!")
                })?
                .to_str()
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Invalid UTF-8 in file name",
                    )
                })?
                .to_string();
            sources.push((resolved.dir, resolved.rel, filename));
        }

        let target_dir = resolved_target.dir.try_clone()?;
        let target_rel = resolved_target.rel.clone();

        let zip_file_size = tokio::task::spawn_blocking(move || {
            let file = target_dir.create(&target_rel)?;
            let mut zip_writer = ZipWriter::new(file);
            let options: zip::write::FileOptions<()> =
                zip::write::FileOptions::default().compression_method(CompressionMethod::Deflated);

            for (dir, rel, filename) in &sources {
                let mut input_file = dir.open(rel)?;
                let mut buffer = Vec::new();
                input_file.read_to_end(&mut buffer)?;

                zip_writer.start_file(filename, options)?;
                zip_writer.write_all(&buffer)?;
                zip_writer.flush()?;
            }

            zip_writer.finish()?;
            let metadata = target_dir.metadata(&target_rel)?;
            Ok::<u64, std::io::Error>(metadata.len())
        })
        .await
        .map_err(std::io::Error::other)??;

        let result_message = format!(
            "Successfully compressed {} {} into '{}' ({}).",
            file_count,
            if file_count == 1 { "file" } else { "files" },
            resolved_target.display.display(),
            format_bytes_size(zip_file_size)
        );
        Ok(result_message)
    }
}
