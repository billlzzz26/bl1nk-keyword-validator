# TODO: Keyword Usage Scanner

## Task List
- [x] Add scanner.rs module - file/folder content scanning and keyword detection
- [x] Add UsageStats struct - count keyword occurrences per file
- [x] Add count_usage_in_path() function - scan a single file for keywords
- [x] Add count_usage_in_dir() function - recursive directory scanning
- [x] Add get_usage_stats() function - aggregate stats across all files
- [x] Add CLI command 'bl1nk-keyword usage <path>' for usage counting
- [x] Add tests for scanner and usage stats

## Improvements Needed
- [ ] Add relationship fields (related_to, parent_child) for relative keyword analysis
- [ ] Use schema errorMessages for meaningful validation errors
- [ ] Document supported/unsupported file extensions
- [x] Run fmt + clippy checks

## Supported File Extensions

### Supported (text-based)
- Source code: `rs`, `py`, `js`, `ts`, `go`, `java`, `cpp`, `c`, `h`, `sh`, `rb`, `php`, `swift`, `kt`, `scala`
- Docs: `md`, `txt`, `rst`
- Config: `json`, `yaml`, `yml`, `toml`, `xml`
- Web: `html`, `css`, `scss`, `vue`, `svelte`

### Unsupported (binary/compiled)
- Images: `png`, `jpg`, `gif`, `svg` (skip)
- Binary: `exe`, `dll`, `so`, `dylib`, `bin` (skip)
- Compressed: `zip`, `tar`, `gz`, `rar` (skip)

## Relationship Analysis Plan
- Add `related_to` array field in entry schema
- Add `parent_child` for hierarchical relationships
- `find_related(keyword_id)` - returns related keywords
- `get_correlation_matrix()` - keyword co-occurrence stats

## Error Messages Usage
Schema has `errorMessages` in validation section - scanner should use these for meaningful errors like:
- `MAPPING_NOT_FOUND`: No mapping found for '{term}' in language '{language}'
- `CONFIDENCE_OUT_OF_RANGE`: Confidence score must be 0-1, got {value}