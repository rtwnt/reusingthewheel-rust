use std::collections::HashSet;
use std::{fmt, fs};
use comrak::{Arena, ComrakExtensionOptions, ComrakOptions, ComrakParseOptions, ComrakRenderOptions, format_html, parse_document};
use comrak::nodes::{AstNode, NodeValue};
use maud::html;
use serde::Deserialize;
use fmt::{Display, Formatter, Result};

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

#[derive(Deserialize)]
struct PageConfig {
    title: String,
    date: String,
    #[serde(default = "HashSet::new")]
    categories: HashSet<String>,
    #[serde(default = "HashSet::new")]
    projects: HashSet<String>
}

impl Display for PageConfig {
    fn fmt(&self, formatter: &mut Formatter) -> Result {
        write!(formatter, "PageConfig{{\n\ttitle: {}\n\tdate: {}\n\tcategories: {}\n\tprojects: {}\n}}",
               self.title, self.date, itertools::join(&self.categories, ", "), itertools::join(&self.projects, ", "))
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

fn get_parsed_yaml_front_matter(text: &mut Vec<u8>) -> Option<PageConfig> {
    let yaml_string = &String::from_utf8_lossy(text).to_string();
    let rest = remove_suffix(yaml_string, "---\n");
    serde_yaml::from_str(rest).unwrap()
}

pub fn print_example_html_using_comrak(filename: &str) {
    let contents = fs::read_to_string(filename)
        .expect("Something went wrong reading the file");

    // The returned nodes are created in the supplied Arena, and are bound by its lifetime.
    let arena = Arena::new();

    let options = ComrakOptions {
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

    println!("{}", String::from_utf8(html).unwrap());
}

fn main() {
    print_example_html_using_maud();
    // Reference links work in comrak, but including Hugo-style references breaks them
    print_example_html_using_comrak("src/content/posts/a-new-blog-engine-project.md");
}
