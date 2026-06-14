//! Filesystem generated-output adapter.

use std::fs;
use std::path::Path;

use sqlcomp_app::GeneratedFileWriter;
use sqlcomp_core as core;

/// Filesystem-backed generated file writer.
#[derive(Clone, Copy, Debug, Default)]
pub struct FileSystemGeneratedFileWriter;

impl GeneratedFileWriter for FileSystemGeneratedFileWriter {
    fn write(&self, files: &core::GeneratedFiles) -> core::DiagnosticResult<()> {
        for file in files.files() {
            write_generated_file(file)?;
        }

        Ok(())
    }
}

fn write_generated_file(file: &core::GeneratedFile) -> core::DiagnosticResult<()> {
    if let Some(parent) = file
        .path()
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            file_error(
                format!(
                    "failed to create generated output directory `{}`: {error}",
                    parent.display()
                ),
                parent,
            )
        })?;
    }

    fs::write(file.path(), file.contents()).map_err(|error| {
        file_error(
            format!(
                "failed to write generated file `{}`: {error}",
                file.path().display()
            ),
            file.path(),
        )
    })
}

fn file_error(message: impl Into<String>, path: &Path) -> core::DiagnosticReport {
    core::DiagnosticReport::new(
        core::Diagnostic::error(message).with_location(core::SourceLocation::for_path(path)),
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::FileSystemGeneratedFileWriter;
    use sqlcomp_app::GeneratedFileWriter;
    use sqlcomp_core as core;

    #[test]
    fn writes_generated_files_and_creates_parent_directories() {
        let temp_dir = unique_temp_dir("sqlcomp-output-writer");
        let output_path = temp_dir.join("src/generated/sqlcomp/sql/users.ts");
        let files = core::GeneratedFiles::new(vec![core::GeneratedFile::new(
            output_path.clone(),
            "generated".to_owned(),
        )]);

        FileSystemGeneratedFileWriter
            .write(&files)
            .expect("generated files should be written");

        assert_eq!(
            std::fs::read_to_string(&output_path).expect("generated file should exist"),
            "generated"
        );

        std::fs::remove_dir_all(temp_dir).expect("temp output dir should be removed");
    }

    #[test]
    fn overwrites_existing_generated_files() {
        let temp_dir = unique_temp_dir("sqlcomp-output-overwrite");
        let output_path = temp_dir.join("src/generated/sqlcomp/sql/users.ts");
        std::fs::create_dir_all(
            output_path
                .parent()
                .expect("output path should have a parent directory"),
        )
        .expect("output parent dir should be created");
        std::fs::write(&output_path, "old").expect("old generated file should be written");
        let files = core::GeneratedFiles::new(vec![core::GeneratedFile::new(
            output_path.clone(),
            "new".to_owned(),
        )]);

        FileSystemGeneratedFileWriter
            .write(&files)
            .expect("generated files should be overwritten");

        assert_eq!(
            std::fs::read_to_string(&output_path).expect("generated file should exist"),
            "new"
        );

        std::fs::remove_dir_all(temp_dir).expect("temp output dir should be removed");
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();

        std::env::temp_dir().join(format!("{prefix}-{}-{unique}", std::process::id()))
    }
}
