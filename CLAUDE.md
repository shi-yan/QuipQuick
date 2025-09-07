# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## QuipQuick Static Blog Generator

QuipQuick is a Rust-based static blog generator that converts markdown posts into HTML with support for math equations, YouTube embeds, image sources, and RSS feeds.

## Development Commands

### Build and Run
```bash
cargo build          # Build the project
cargo run -- <cmd>   # Run QuipQuick commands locally
cargo install --path . # Install locally
```

### Testing
```bash
cargo test           # Run unit tests
cargo check          # Check for compilation errors
cargo clippy         # Run linter
cargo fmt            # Format code
```

### QuipQuick Commands
```bash
quipquick new                    # Create new blog boilerplate
quipquick write                  # Create new post
quipquick pub -t <target_dir>    # Generate blog to target directory
```

## Architecture

### Core Components

**Command Structure (`src/main.rs`)**
- CLI built with `clap` providing three main commands: `new`, `write`, `pub`
- Each command delegates to specialized modules

**Post Processing Pipeline**
1. **Frontmatter parsing** (`src/frontmatter.rs`) - extracts YAML metadata from markdown
2. **Markdown to HTML conversion** (`src/md2html.rs`) - custom renderer extending CommonMark
3. **Template rendering** (`src/publish.rs`) - uses Handlebars for HTML generation

**Key Modules:**
- `src/post.rs` - Post data structure and serialization
- `src/new.rs` - Blog initialization and template population
- `src/write.rs` - New post creation workflow
- `src/publish.rs` - Main publishing logic with RSS generation
- `src/gallery.rs` - Gallery feature support

### Template System

**Templates** (`template_src/`)
- `index.html` - Blog homepage template
- `post.html` - Individual post template  
- `gallery.html` - Gallery page template
- `style.css` - Default stylesheet
- Templates use Handlebars syntax with blog metadata injection

### Markdown Extensions

QuipQuick extends CommonMark with:
- YouTube video embedding: ````youtube\n<video_id>\n````
- Image sources: `![alt|source1|source2](url)`
- Math equation support (inline and block)
- Footnote support

### Content Structure

**Blog Configuration**
- `quipquick.toml` - Main config file (not present in source, created by `new` command)
- Contains blog metadata, content folder list, and settings

**Content Organization**
- Each post lives in its own folder with `content.md` and assets
- Frontmatter in markdown files defines post metadata
- Images are automatically processed and resized

### Data Flow

1. `pub` command reads `quipquick.toml` configuration
2. Processes each content folder listed in config
3. Parses markdown with frontmatter extraction
4. Converts markdown to HTML with custom extensions
5. Applies Handlebars templates with post/blog data
6. Generates RSS feed and copies assets
7. Outputs complete static site to target directory

### Dependencies

Key Rust crates:
- `handlebars` - Template engine
- `markdown` - CommonMark parser with extensions
- `clap` - CLI argument parsing
- `serde`/`toml` - Configuration serialization
- `image` - Image processing
- `rss` - RSS feed generation