use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::io;

fn main() {
    let mut args = std::env::args_os();
    let exe         = PathBuf::from(args.next().unwrap_or_else(|| "bookmarks2md".into()));
    let profile_dir = PathBuf::from(args.next().unwrap_or_else(|| die_usage(&exe)));
    let output_md   = PathBuf::from(args.next().unwrap_or_else(|| die_usage(&exe)));

    let mut chrome_bookmarks = profile_dir.clone();
    chrome_bookmarks.push("Default");
    chrome_bookmarks.push("Bookmarks"); // no extension
    match std::fs::read_to_string(&chrome_bookmarks) {
        Ok(json) => run(&json, &output_md).expect("failed to run"),
        Err(err) => {
            eprintln!("Error reading {chrome_bookmarks:?}: {err:?}");
            std::process::exit(1);
        },
    }
}

fn escape<'s>(text: &'s str) -> Cow<'s, str> {
    // TODO: escape title/url for markdown purpouses?
    text.into()
}

fn run(json: &str, output_md: &Path) -> io::Result<()> {
    use std::io::Write;
    let parsed = bookmarks::parse_json(&json);
    let mut output_md = std::io::BufWriter::new(std::fs::File::create(&output_md).expect("unable to open output.md"));
    for e in parsed.iter() {
        let title = escape(&e.title);
        if let Some(url) = &e.url {
            let url = escape(url);
            if title == "" {
                writeln!(output_md, "# <{url}>")?;
            } else {
                writeln!(output_md, "# [{title}]({url})")?;
            }
        } else if title == "" {
            writeln!(output_md, "# Untitled")?;
        } else {
            writeln!(output_md, "# {title}")?;
        }
        for child in e.children.iter() {
            write_entry("*   ", child, &mut output_md)?;
        }
        writeln!(output_md)?;
    }
    Ok(())
}

fn write_entry(prefix: &str, e: &bookmarks::Entry, output_md: &mut impl io::Write) -> io::Result<()> {
    let title = escape(&e.title);
    write!(output_md, "{prefix}")?;
    if let Some(url) = &e.url {
        let url = escape(url);
        if title == "" {
            write!(output_md, "<{url}>")?;
        } else {
            write!(output_md, "[{title}]({url})")?;
        }
    } else {
        if title == "" {
            write!(output_md, "<span style=\"opacity: 50%\">(Untitled)</span>")?;
        } else {
            write!(output_md, "{title}")?;
        }
    }
    writeln!(output_md)?;
    if !e.children.is_empty() {
        let prefix = format!("    {prefix}");
        for e in e.children.iter() {
            write_entry(&prefix, e, output_md)?;
        }
    }
    Ok(())
}

fn die_usage(exe: &Path) -> ! {
    eprintln!("Usage:");
    eprintln!("    {exe}  input/profile/dir/  output/bookmarks.md", exe = exe.display());
    std::process::exit(1);

    // TODO: determine examples baed on:
    // https://chromium.googlesource.com/chromium/src/+/HEAD/docs/user_data_dir.md
    //
    // Windows
    // * [Chrome] %LOCALAPPDATA%\Google\Chrome\User Data
    // * [Chrome Beta] %LOCALAPPDATA%\Google\Chrome Beta\User Data
    // * [Chrome Canary] %LOCALAPPDATA%\Google\Chrome SxS\User Data
    // * [Chrome for Testing] %LOCALAPPDATA%\Google\Chrome for Testing\User Data
    // * [Chromium] %LOCALAPPDATA%\Chromium\User Data
    //
    // Linux
    // * [Chrome Stable] ~/.config/google-chrome
    // * [Chrome Beta] ~/.config/google-chrome-beta
    // * [Chrome Dev] ~/.config/google-chrome-unstable
    // * [Chrome for Testing] ~/.config/google-chrome-for-testing
    // * [Chromium] ~/.config/chromium
}

pub mod bookmarks {
    pub struct Entry {
        // favicon?
        pub title:      String,
        pub url:        Option<String>,
        pub children:   Vec<Entry>,
    }

    pub fn parse_json(json: &str) -> Vec<Entry> {
        use serde_json::*;
        let mut entries = Vec::new();
        let root : Value = from_str(json).expect("unable to parse json root");

        if let Some(bookmark_bar_children) = root["roots"]["bookmark_bar"]["children"].as_array() {
            let mut e = Entry {
                title:      "Bookmark Bar".into(),
                url:        None,
                children:   Vec::new()
            };
            e.children.reserve(bookmark_bar_children.len());
            for child in bookmark_bar_children.iter() {
                e.children.push(parse_child(child));
            }
            entries.push(e);
        }

        entries
    }

    fn parse_child(child: &serde_json::Value) -> Entry {
        let mut e = Entry {
            title:      child["name"].as_str().unwrap_or("").into(),
            url:        child["url"].as_str().map(|s| s.into()),
            children:   Vec::new(),
        };
        if let Some(children) = child["children"].as_array() {
            e.children.reserve(children.len());
            for child in children.iter() {
                e.children.push(parse_child(child));
            }
        }
        e
    }
}
