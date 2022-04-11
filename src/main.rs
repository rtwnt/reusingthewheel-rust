use std::collections::{HashMap, HashSet};
use std::{fmt, fs};
use comrak::{Arena, ComrakExtensionOptions, ComrakOptions, ComrakParseOptions, ComrakRenderOptions, format_html, parse_document};
use comrak::nodes::{AstNode, NodeValue};
use maud::html;
use serde::{Deserialize, Deserializer};
use fmt::{Display, Formatter, Result};
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use chrono::{Datelike, DateTime, MIN_DATETIME, TimeZone, Utc};
use itertools::Itertools;
use walkdir::WalkDir;

fn print_example_html_using_maud() {
    let example_html = html! {
        h1 { "Hello, world!" }
        p.intro {
            "This is an example of the "
            a href="https://github.com/lambda-fairy/maud" { "Maud" }
            " template language."
        }
    };

    println!("{}", example_html.into_string());
}

const FORMAT: &str = "%Y-%m-%dT%H:%M";

fn deserialize_date_with_format<'de, D>(deserializer: D) -> std::result::Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Utc.datetime_from_str(&s, FORMAT)
        .map_err(serde::de::Error::custom)
}

fn deserialize_option_with_date_with_format<'de, D>(deserializer: D) -> std::result::Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Wrapper(#[serde(deserialize_with = "deserialize_date_with_format")] DateTime<Utc>);

    let v = Option::deserialize(deserializer)?;
    Ok(v.map(|Wrapper(a)| a))
}

#[derive(Deserialize)]
struct PageConfig {
    title: String,
    #[serde(default, deserialize_with = "deserialize_option_with_date_with_format")]
    date: Option<DateTime<Utc>>,
    #[serde(default = "HashSet::new")]
    categories: HashSet<String>,
    #[serde(default = "HashSet::new")]
    projects: HashSet<String>
}

impl Display for PageConfig {
    fn fmt(&self, formatter: &mut Formatter) -> Result {
        write!(formatter, "PageConfig{{\n\ttitle: {}\n\tdate: {}\n\tcategories: {}\n\tprojects: {}\n}}",
               self.title, self.date.unwrap_or(MIN_DATETIME).to_string(), itertools::join(&self.categories, ", "), itertools::join(&self.projects, ", "))
    }
}

struct Page {
    html_file_path: PathBuf,
    config: PageConfig,
}

struct HtmlContent {
    page: Page,
    content: Vec<u8>
}

fn iter_nodes<'a, F>(node: &'a AstNode<'a>, f: &mut F)
    where F : FnMut(&'a AstNode<'a>) {
    f(node);
    for c in node.children() {
        iter_nodes(c, f);
    }
}

fn remove_suffix<'a>(s: &'a str, suffix: &str) -> &'a str {
    match s.strip_suffix(suffix) {
        Some(s) => s,
        None => s
    }
}

fn remove_prefix<'a>(s: &'a str, prefix: &str) -> &'a str {
    match s.strip_prefix(prefix) {
        Some(s) => s,
        None => s
    }
}

const YAML_SEPARATOR: &str = "---\n";

fn get_parsed_yaml_front_matter(text: &mut Vec<u8>) -> Option<PageConfig> {
    let yaml_string = &String::from_utf8_lossy(text).to_string();
    let mut rest = remove_suffix(yaml_string, YAML_SEPARATOR);
    rest = remove_prefix(rest, YAML_SEPARATOR);
    serde_yaml::from_str(rest).unwrap()
}
fn prepare_options() -> ComrakOptions {
    return ComrakOptions {
        extension: ComrakExtensionOptions {
            strikethrough: false,
            tagfilter: false,
            table: true,
            autolink: false,
            tasklist: false,
            superscript: false,
            header_ids: Some("abc".to_string()),
            footnotes: true,
            description_lists: false,
            front_matter_delimiter: Some("---".to_string()),
        },
        parse: ComrakParseOptions {
            smart: false,
            default_info_string: Some("abc".to_string()),
        },
        render: ComrakRenderOptions {
            hardbreaks: false,
            github_pre_lang: false,
            width: 123456,
            unsafe_: false,
            escape: false,
        },
    };
}

fn get_html_file_path(markdown_file_path: &Path) -> PathBuf {
    let without_prefix: &str;
    match markdown_file_path.to_str().unwrap().strip_prefix("content") {
        Some(s) => without_prefix = s,
        None => panic!("Missing \"content\" prefix from {}", markdown_file_path.display())
    }
    let file_path_elements = without_prefix.rsplit_once("/").unwrap();
    let html_file_name: String;
    match file_path_elements.1.strip_suffix(".md") {
        Some(s) => html_file_name = s.to_string() + ".html",
        None => panic!("Missing \".md\" suffix from {}", file_path_elements.1)
    }
    let html_file_path = "public".to_string() + file_path_elements.0 + "/" + &html_file_name;
    return PathBuf::from(&html_file_path);
}

fn parse_page_data(filename: &Path, html_file_path: PathBuf, options: &ComrakOptions) -> HtmlContent {
    println!("{}", filename.display());
    let contents = fs::read_to_string(filename)
        .expect("Something went wrong reading the file");
    // The returned nodes are created in the supplied Arena, and are bound by its lifetime.
    let arena = Arena::new();
    let root = parse_document(
        &arena,
        &contents,
        &options);
    let mut html = vec![];
    format_html(root, &options, &mut html).unwrap();
    let mut article_config: Option<PageConfig> = None;
    iter_nodes(root, &mut|node| {
        match &mut node.data.borrow_mut().value {
            &mut NodeValue::FrontMatter(ref mut text) => {
                article_config = get_parsed_yaml_front_matter(text);
            },
            _ => (),
        }
    });
    let page = Page{
        html_file_path,
        config: article_config.unwrap(),
    };
    return HtmlContent {
        page,
        content: html
    }
}

fn save_to_html_file(html: &HtmlContent) {
    let html_file_path = &html.page.html_file_path;
    let parent_dir = html_file_path.parent().unwrap();
    match create_dir_all(parent_dir) {
        Ok(_t) => {},
        Err(error) => panic!("Error while creating directory {}: {}", parent_dir.display(), error),
    };
    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(html_file_path) {
        Err(error) => panic!("Couldn't create {}: {}", html_file_path.display(), error),
        Ok(file) => file,
    };
    match file.write_all(html.content.as_ref()) {
        Err(error) => panic!("Couldn't write to {}: {}", html_file_path.display(), error),
        Ok(_) => println!("Successfully wrote to {}", html_file_path.display()),
    }
}

fn main() {
    print_example_html_using_maud();

    let options = prepare_options();
    let mut pages_by_categories: HashMap<String, Vec<&Page>> = HashMap::new();
    let mut pages_by_projects: HashMap<String, Vec<&Page>> = HashMap::new();
    let mut pages_by_year: HashMap<String, Vec<&Page>> = HashMap::new();

    let pages = WalkDir::new("content")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_type().is_dir())
        .map(|entry| {
            let html_path = get_html_file_path(entry.path());
            let page = parse_page_data(entry.path(), html_path, &options);
            save_to_html_file(&page);
            return page.page;
        }).collect_vec();

    pages.iter()
        .for_each(
            |page| {
                page.config
                    .categories
                    .iter()
                    .for_each(
                        |category|
                            pages_by_categories.entry(category.to_string())
                                .or_insert_with(Vec::new)
                                .push(&page)
                    );

                page.config
                    .projects
                    .iter()
                    .for_each(
                        |project|
                            pages_by_projects.entry(project.to_string())
                                .or_insert_with(Vec::new)
                                .push(&page)
                    );

                match page.config.date {
                    Some(datetime) => {
                        println!("Got year {}", datetime.year());
                        pages_by_year.entry(datetime.year().to_string()).or_insert_with(Vec::new).push(&page);
                    },
                    None => println!("Article {} does not have a date", page.config.title),
                }
            }
        );
    println!("DONE");
}
