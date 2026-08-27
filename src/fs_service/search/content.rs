use crate::{
    error::ServiceResult,
    fs_service::{FileSystemService, utils::escape_regex},
};
use grep::{
    matcher::{Match, Matcher},
    regex::RegexMatcherBuilder,
    searcher::{BinaryDetection, Searcher, sinks::UTF8},
};
use std::path::{Path, PathBuf};

const SNIPPET_MAX_LENGTH: usize = 200;
const SNIPPET_BACKWARD_CHARS: usize = 30;

/// Represents a single match found in a file's content.
#[derive(Debug, Clone)]
pub struct ContentMatchResult {
    /// The line number where the match occurred (1-based).
    pub line_number: u64,
    pub start_pos: usize,
    /// The line of text containing the match.
    /// If the line exceeds 255 characters (excluding the search term), only a truncated portion will be shown.
    pub line_text: String,
}

/// Represents all matches found in a specific file.
#[derive(Debug, Clone)]
pub struct FileSearchResult {
    /// The path to the file where matches were found.
    pub file_path: PathBuf,
    /// All individual match results within the file.
    pub matches: Vec<ContentMatchResult>,
}

impl FileSystemService {
    // Searches the content of a file for occurrences of the given query string.
    ///
    /// This method searches the file specified by `file_path` for lines matching the `query`.
    /// The search can be performed as a regular expression or as a literal string,
    /// depending on the `is_regex` flag.
    ///
    /// If matched line is larger than 255 characters, a snippet will be extracted around the matched text.
    ///
    pub async fn content_search(
        &self,
        query: &str,
        file_path: impl AsRef<Path>,
        is_regex: Option<bool>,
    ) -> ServiceResult<Option<FileSearchResult>> {
        let resolved = self.resolve(file_path.as_ref()).await?;

        let query = if is_regex.unwrap_or_default() {
            query.to_string()
        } else {
            escape_regex(query)
        };

        let matcher = RegexMatcherBuilder::new()
            .case_insensitive(true)
            .build(query.as_str())?;

        let mut searcher = Searcher::new();
        let mut result = FileSearchResult {
            file_path: resolved.display.clone(),
            matches: vec![],
        };

        searcher.set_binary_detection(BinaryDetection::quit(b'\x00'));

        // Open through the confined directory handle and search the reader,
        // so the search cannot escape the allowed directory via a symlink.
        let file = resolved.dir.open(&resolved.rel)?.into_std();

        searcher.search_reader(
            &matcher,
            file,
            UTF8(|line_number, line| {
                let actual_match = matcher.find(line.as_bytes())?.unwrap();

                result.matches.push(ContentMatchResult {
                    line_number,
                    start_pos: actual_match.start(),
                    line_text: self.extract_snippet(line, actual_match, None, None),
                });
                Ok(true)
            }),
        )?;

        if result.matches.is_empty() {
            return Ok(None);
        }

        Ok(Some(result))
    }

    /// Extracts a snippet from a given line of text around a match.
    pub fn extract_snippet(
        &self,
        line: &str,
        match_result: Match,
        max_length: Option<usize>,
        backward_chars: Option<usize>,
    ) -> String {
        let max_length = max_length.unwrap_or(SNIPPET_MAX_LENGTH);
        let backward_chars = backward_chars.unwrap_or(SNIPPET_BACKWARD_CHARS);

        // Calculate the number of leading whitespace bytes to adjust for trimmed input
        let start_pos = line.len() - line.trim_start().len();
        // Trim leading and trailing whitespace from the input line
        let line = line.trim();

        // Calculate the desired start byte index by adjusting match start for trimming and backward chars
        let desired_start = (match_result.start() - start_pos).saturating_sub(backward_chars);

        // Find the nearest valid UTF-8 character boundary at or after desired_start
        let snippet_start = line
            .char_indices()
            .map(|(i, _)| i)
            .find(|&i| i >= desired_start)
            .unwrap_or(desired_start.min(line.len()));
        // Initialize a counter for tracking characters to respect max_length
        let mut char_count = 0;

        // Calculate the desired end byte index by counting max_length characters from snippet_start
        let desired_end = line[snippet_start..]
            .char_indices()
            .take(max_length + 1)
            .find(|&(_, _)| {
                char_count += 1;
                char_count > max_length
            })
            .map(|(i, _)| snippet_start + i)
            .unwrap_or(line.len());

        // Ensure snippet_end is a valid UTF-8 character boundary at or after desired_end
        let snippet_end = line
            .char_indices()
            .map(|(i, _)| i)
            .find(|&i| i >= desired_end)
            .unwrap_or(line.len());

        // Cap snippet_end to avoid exceeding the string length
        let snippet_end = snippet_end.min(line.len());

        // Extract the snippet from the trimmed line using the calculated byte indices
        let snippet = &line[snippet_start..snippet_end];

        let mut result = String::new();
        // Add leading ellipsis if the snippet doesn't start at the beginning of the trimmed line
        if snippet_start > 0 {
            result.push_str("...");
        }

        result.push_str(snippet);

        // Add trailing ellipsis if the snippet doesn't reach the end of the trimmed line
        if snippet_end < line.len() {
            result.push_str("...");
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn search_files_content(
        &self,
        root_path: impl AsRef<Path>,
        pattern: &str,
        query: &str,
        is_regex: bool,
        exclude_patterns: Option<Vec<String>>,
        min_bytes: Option<u64>,
        max_bytes: Option<u64>,
    ) -> ServiceResult<Vec<FileSearchResult>> {
        let entries = self
            .search_files_iter(
                root_path.as_ref(),
                pattern.to_string(),
                exclude_patterns.unwrap_or_default(),
                min_bytes,
                max_bytes,
            )
            .await?;

        let mut results: Vec<FileSearchResult> = Vec::new();
        for entry in entries.into_iter().filter(|e| e.is_file()) {
            if let Ok(Some(result)) = self.content_search(query, &entry.display, Some(is_regex)).await {
                results.push(result);
            }
        }
        Ok(results)
    }
}
