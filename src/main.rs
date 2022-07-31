mod reusingthewheel_theme;

use std::collections::{HashMap, HashSet};
use std::{fmt, fs};
use comrak::{Arena, ComrakExtensionOptions, ComrakOptions, ComrakParseOptions, ComrakRenderOptions, format_html, parse_document};
use comrak::nodes::{AstNode, NodeValue};
use maud::html;
use serde::{Deserialize, Deserializer};
use fmt::{Display, Formatter, Result};
use std::cmp::Reverse;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use chrono::{Datelike, DateTime, MIN_DATETIME, TimeZone, Utc};
use itertools::Itertools;
use walkdir::WalkDir;
use crate::reusingthewheel_theme::reusingthewheel_theme::{archive, category_list, single_page};

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
    #[serde(default = "String::new")]
    path: String,
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

pub struct Page {
    html_file_path: PathBuf,
    config: PageConfig,
}

struct HtmlContent {
    page: Page,
    content: Vec<u8>
}

pub struct Link {
    url: String,
    title: String,
}

pub struct Website {
    base_url: String,
    title: String,
    menu_items: Vec<Link>,
    description: String,
    author: String,
    year: String,
}

impl Website {
    pub fn get_category_links_for_page(&self, page: &Page) -> Vec<Link> {
        return page.config.categories.iter()
            .map(|tag| {
                Link {
                    url: self.base_url.to_string() + "/tags/" + tag,
                    title: tag.to_owned()
                }
            }).collect_vec()
    }

    pub fn get_all_category_links(&self) -> Vec<Link> {
        return vec![]
    }
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

fn prepare_page_data(filename: &Path, options: &ComrakOptions) -> HtmlContent {
    let html_file_path = get_html_file_path(filename);
    println!("{}", filename.display());
    let markdown_content = parse_document_content(filename, options);
    let config = PageConfig {
        title: markdown_content.original_config.title,
        path: prepare_path(markdown_content.original_config.path, &html_file_path),
        date: markdown_content.original_config.date,
        categories: markdown_content.original_config.categories,
        projects: markdown_content.original_config.projects
    };
    let page = Page {
        html_file_path,
        config,
    };
    return HtmlContent {
        page,
        content: markdown_content.rendered_html
    }
}

fn prepare_path(path: String, html_file_path: &PathBuf) -> String {
    return if path.is_empty() {
        html_file_path.to_str().unwrap().to_owned() } else { path }
}

struct MarkdownContent {
    original_config: PageConfig,
    rendered_html: Vec<u8>
}

fn parse_document_content(filename: &Path, options: &ComrakOptions) -> MarkdownContent {
    let contents = fs::read_to_string(filename)
        .expect("Something went wrong reading the file");
    // The returned nodes are created in the supplied Arena, and are bound by its lifetime.
    let arena = Arena::new();
    let root = parse_document(&arena, &contents, &options);
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
    return MarkdownContent {
        original_config: article_config.unwrap(),
        rendered_html: html
    }
}

fn save_to_html_file(html: &HtmlContent) {
    save_to_path(&html.page.html_file_path, String::from_utf8(html.content.to_owned()).unwrap());
}

fn save_to_path(path: &PathBuf, content: String) {
    let parent_dir = path.parent().unwrap();
    match create_dir_all(parent_dir) {
        Ok(_t) => {},
        Err(error) => panic!("Error while creating directory {}: {}", parent_dir.display(), error),
    };
    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(path) {
        Err(error) => panic!("Couldn't create {}: {}", path.display(), error),
        Ok(file) => file,
    };
    match file.write_all(content.as_ref()) {
        Err(error) => panic!("Couldn't write to {}: {}", path.display(), error),
        Ok(_) => println!("Successfully wrote to {}", path.display()),
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum PageType {
    POST,
    PAGE
}

fn to_map_by_date(pages: Vec<&Page>) -> Vec<(String, Vec<&Page>)> {
    let mut pages_by_year: HashMap<String, Vec<&Page>> = HashMap::new();
    pages.iter().for_each(|page| {
        match page.config.date {
            Some(datetime) => {
                println!("Got year {}", datetime.year());
                pages_by_year.entry(datetime.year().to_string()).or_insert_with(Vec::new).push(&page);
            },
            None => println!("Article {} does not have a date", page.config.title),
        }
    });

    let result: Vec<_> = pages_by_year.into_iter()
        .sorted_by_key(|item| Reverse(item.0.to_owned()))
        .collect();

    return result
}

fn main() {
    print_example_html_using_maud();

    let options = prepare_options();
    let mut pages_by_categories: HashMap<String, Vec<&Page>> = HashMap::new();
    let mut pages_by_projects: HashMap<String, Vec<&Page>> = HashMap::new();
    let mut pages_by_year: HashMap<String, Vec<&Page>> = HashMap::new();
    let mut pages_by_type: HashMap<PageType, Vec<&Page>> = HashMap::new();

    let pages = WalkDir::new("content")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_type().is_dir())
        .map(|entry| {
            let html_content = prepare_page_data(entry.path(), &options);
            save_to_html_file(&html_content);
            return html_content.page;
        }).collect_vec();

    let website = Website {
        base_url: "https://reusingthewheel.net".to_string(),
        title: "Reusing the wheel".to_string(),
        menu_items: Vec::new(),
        description: "A blog about my programming hobby".to_string(),
        author: "Piotr Rusin".to_string(),
        year: "2022".to_string(),
    };

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

                let mut page_type = PageType::PAGE;
                if page.html_file_path.starts_with("public/posts") {
                    page_type = PageType::POST;
                }
                pages_by_type.entry(page_type)
                    .or_insert_with(Vec::new)
                    .push(&page);

                let contents = fs::read_to_string(page.html_file_path.to_owned())
                    .expect("Something went wrong reading the file");
                save_to_path(&page.html_file_path, single_page(&website, &page, contents).into_string())
            }
        );
    let sorted_pages_by_year = pages_by_year.into_iter()
        .sorted_by_key(|item| Reverse(item.0.to_owned()))
        .collect();
    save_to_path(
        &PathBuf::from("public/index.html".to_owned()),
        archive(&website, "Archive", sorted_pages_by_year).into_string()
    );

    // CATEGORIES

    let all_category_links = pages_by_categories.keys()
        .sorted_by_key(|category| { category.to_owned() })
        .map(|category| {
            Link {
                url: website.base_url.to_string() + "/categories/" + &category.to_lowercase(),
                title: category.to_owned()
            }
        }).collect_vec();
    save_to_path(
        &PathBuf::from("public/categories/index.html".to_owned()),
        category_list(&website, "Categories", all_category_links).into_string()
    );
    pages_by_categories.iter()
        .for_each(|(category, pages)| {
            let path = "Categories: ".to_owned() + category;
            save_to_path(
                &PathBuf::from("public/categories/".to_owned() + &category.to_lowercase() + "/index.html"),
                archive(&website, &path, to_map_by_date(pages.to_owned())).into_string()
            );
        });

    // PROJECTS

    let all_project_links = pages_by_projects.keys()
        .sorted_by_key(|category| { category.to_owned() })
        .map(|category| {
            Link {
                url: website.base_url.to_string() + "/projects/" + &category.to_lowercase(),
                title: category.to_owned()
            }
        }).collect_vec();
    save_to_path(
        &PathBuf::from("public/projects/index.html".to_owned()),
        category_list(&website, "Projects", all_project_links).into_string()
    );
    pages_by_projects.iter()
        .for_each(|(category, pages)| {
            let path = "Projects: ".to_owned() + category;
            save_to_path(
                &PathBuf::from("public/projects/".to_owned() + &category.to_lowercase() + "/index.html"),
                archive(&website, &path, to_map_by_date(pages.to_owned())).into_string()
            );
        });

    println!("DONE");
}
