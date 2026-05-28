# TODO: Keyword Usage Scanner

## Task List
- [x] Add scanner.rs module - file/folder content scanning and keyword detection
- [x] Add UsageStats struct - count keyword occurrences per file
- [x] Add count_usage_in_path() function - scan a single file for keywords
- [x] Add count_usage_in_dir() function - recursive directory scanning
- [x] Add get_usage_stats() function - aggregate stats across all files
- [x] Add CLI command 'bl1nk-keyword usage <path>' for usage counting
- [x] Add tests for scanner and usage stats

## Notes
- Support recursive directory scanning
- Count keyword occurrences per file: `{ "keyword_id": { "file.rs": count } }`
- Use existing KeywordRegistry for comparison

## Status: COMPLETE - 7/7 tasks finished

## Files Modified
- crate/core/Cargo.toml - added walkdir dependency
- crate/core/src/scanner.rs - new module (count_usage_in_path, count_usage_in_dir, get_total_usage, get_top_keywords)
- crate/core/src/lib.rs - added scanner re-exports
- crate/cli/src/main.rs - added Usage command