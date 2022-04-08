use std::collections::HashSet;
use std::{fmt, fs};
use comrak::{Arena, ComrakExtensionOptions, ComrakOptions, ComrakParseOptions, ComrakRenderOptions, format_html, parse_document};
use comrak::nodes::{AstNode, NodeValue};
use maud::html;
use serde::{Deserialize, Deserializer};
use fmt::{Display, Formatter, Result};
use std::fmt::Error;
use chrono::{DateTime, MIN_DATETIME, TimeZone, Utc};
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
    file_path: String,
    config: PageConfig,
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

fn get_page_struct(filename: &str) -> Page {
    println!("{}", filename);
    let contents = fs::read_to_string(filename)
        .expect("Something went wrong reading the file");

    // The returned nodes are created in the supplied Arena, and are bound by its lifetime.
    let arena = Arena::new();
    let options = prepare_options();
    let root = parse_document(
        &arena,
        &contents,
        &options);

    let mut article_config: Option<PageConfig> = None;

    iter_nodes(root, &mut|node| {
        match &mut node.data.borrow_mut().value {
            &mut NodeValue::FrontMatter(ref mut text) => {
                article_config = get_parsed_yaml_front_matter(text);
            },
            _ => (),
        }
    });

    println!("Article config:\n{}", article_config.as_ref().unwrap());

    let mut html = vec![];
    format_html(root, &options, &mut html).unwrap();

    println!("Article content\n{}", String::from_utf8(html.clone()).unwrap());

    return Page{
        file_path: String::from(filename),
        config: article_config.unwrap(),
        content: html
    }
}

fn main() {
    print_example_html_using_maud();

    for entry in WalkDir::new("src/content")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_type().is_dir()) {
        println!("{}", entry.file_name().to_str().unwrap());
        println!("{}", entry.path().to_str().unwrap());
        let page = get_page_struct(entry.path().to_str().unwrap());
        println!("\n\n{}", page.file_path);
        // println!("\n");
        println!("{}", page.config);
        // println!("\n");
        println!("{}",  String::from_utf8(page.content).unwrap());
    }
}
